use crate::db::get_connection;
use crate::models::entity::{Entity, Strategy};
use crate::models::mapping::MatchInfo;
use crate::models::session::{
    DesensitizeInput, DesensitizeResult, ScanInput, ScanResult, ScanResultItem,
};
use crate::services::ner_service::{label_to_entity_name, ner_scan};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;

/// 正则缓存：避免重复编译相同正则表达式
static REGEX_CACHE: Lazy<Mutex<HashMap<String, Regex>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// 从缓存获取或编译正则
fn get_or_compile_regex(pattern: &str) -> Option<Regex> {
    let mut cache = REGEX_CACHE.lock().unwrap();
    if let Some(re) = cache.get(pattern) {
        return Some(re.clone());
    }
    if let Ok(re) = Regex::new(pattern) {
        cache.insert(pattern.to_string(), re.clone());
        return Some(re);
    }
    None
}

/// 大文件分段处理阈値（超过此大小将分段处理）
const CHUNK_THRESHOLD: usize = 50_000;

// =============================================================
// 扫描阶段：只扫不写库
// =============================================================

pub fn scan(input: ScanInput) -> Result<ScanResult, String> {
    let entities = get_enabled_entities()?;
    if entities.is_empty() {
        return Ok(ScanResult { items: vec![] });
    }

    let matches = scan_content(&input.content, &entities)?;

    // 按 original_value 去重，保留首次出现的扫描项
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut items: Vec<ScanResultItem> = Vec::new();

    // 构建 entity_id -> strategy 映射用于查找策略
    let entity_strategy_map: HashMap<String, String> = entities
        .iter()
        .map(|e| {
            let s = match e.strategy {
                crate::models::entity::Strategy::Empty => "empty".to_string(),
                crate::models::entity::Strategy::RandomReplace => "random_replace".to_string(),
            };
            (e.id.clone(), s)
        })
        .collect();

    for m in &matches {
        if seen.contains(&m.value) {
            continue;
        }
        seen.insert(m.value.clone());
        let strategy = entity_strategy_map
            .get(&m.entity_id)
            .cloned()
            .unwrap_or_else(|| "random_replace".to_string());
        items.push(ScanResultItem {
            original_value: m.value.clone(),
            entity_name: m.entity_name.clone(),
            strategy,
        });
    }

    Ok(ScanResult { items })
}

// =============================================================
// 脱敏阶段：按用户清单执行，不查全局实体配置
// =============================================================

pub fn desensitize(input: DesensitizeInput) -> Result<DesensitizeResult, String> {
    if input.items.is_empty() {
        return Ok(DesensitizeResult {
            session_id: String::new(),
            original_content: input.content.clone(),
            desensitized_content: input.content,
            mapping_count: 0,
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        });
    }

    // 若覆盖旧会话，先删除
    if let Some(ref old_id) = input.session_id {
        if !old_id.is_empty() {
            let _ = delete_session_by_id(old_id);
        }
    }

    // 按用户清单构建替换表：相同原文复用占位符
    let (desensitized_content, mappings) = if input.content.len() > CHUNK_THRESHOLD {
        apply_items_large(&input.content, &input.items)?
    } else {
        apply_items(&input.content, &input.items)?
    };

    let session_id = save_session(&input.content, &desensitized_content, &mappings)?;

    Ok(DesensitizeResult {
        session_id,
        original_content: input.content,
        desensitized_content,
        mapping_count: mappings.len(),
        created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    })
}

/// 基于原文位置的安全替换：先定位所有匹配，从右向左替换，防止占位符被后续替换污染
/// （例如：先替换银行卡号生成 [银行账户/卡号_1]，再替换"银行"时不会破坏占位符内容）
fn replace_by_position(
    content: &str,
    items: &[crate::models::session::DesensitizeItem],
    entity_counters: &mut HashMap<String, usize>,
    value_to_placeholder: &mut HashMap<String, String>,
    mappings: &mut Vec<(String, String, String)>,
) -> String {
    // 1. 找到所有 original_value 在 content 中的字节位置
    let mut occurrences: Vec<(usize, usize, usize)> = Vec::new(); // (start, end, item_idx)
    for (idx, item) in items.iter().enumerate() {
        if item.original_value.is_empty() {
            continue;
        }
        let val = item.original_value.as_str();
        let mut search_start = 0usize;
        while search_start < content.len() {
            match content[search_start..].find(val) {
                Some(pos) => {
                    let abs_start = search_start + pos;
                    let abs_end = abs_start + val.len();
                    occurrences.push((abs_start, abs_end, idx));
                    search_start = abs_end;
                }
                None => break,
            }
        }
    }

    // 2. 去重叠：按 start 升序、end 降序排，保留非重叠（同位置取较长的）
    occurrences.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
    let mut filtered: Vec<(usize, usize, usize)> = Vec::new();
    let mut last_end = 0usize;
    for occ in occurrences {
        if occ.0 >= last_end {
            last_end = occ.1;
            filtered.push(occ);
        }
    }

    // 3. 按首次出现顺序分配占位符编号（保持稳定输出）
    for &(_, _, idx) in &filtered {
        let item = &items[idx];
        if item.strategy == "empty" {
            continue;
        }
        if !value_to_placeholder.contains_key(&item.original_value) {
            let counter = entity_counters
                .entry(item.entity_name.clone())
                .or_insert(0);
            *counter += 1;
            let ph = format!("[{}_{}]", item.entity_name, counter);
            value_to_placeholder.insert(item.original_value.clone(), ph.clone());
            mappings.push((ph, item.original_value.clone(), item.entity_name.clone()));
        }
    }

    // 4. 从右向左替换，确保左侧已替换位置不受影响
    filtered.sort_by(|a, b| b.0.cmp(&a.0));
    let mut result = content.to_string();
    for (start, end, idx) in filtered {
        let item = &items[idx];
        match item.strategy.as_str() {
            "empty" => {
                result.replace_range(start..end, "");
            }
            _ => {
                if let Some(ph) = value_to_placeholder.get(&item.original_value) {
                    let ph = ph.clone();
                    result.replace_range(start..end, &ph);
                }
            }
        }
    }
    result
}

/// 按用户清单直接替换（普通文件）
fn apply_items(
    content: &str,
    items: &[crate::models::session::DesensitizeItem],
) -> Result<(String, Vec<(String, String, String)>), String> {
    let mut mappings: Vec<(String, String, String)> = Vec::new();
    let mut entity_counters: HashMap<String, usize> = HashMap::new();
    let mut value_to_placeholder: HashMap<String, String> = HashMap::new();

    let result = replace_by_position(
        content,
        items,
        &mut entity_counters,
        &mut value_to_placeholder,
        &mut mappings,
    );

    Ok((result, mappings))
}

/// 按用户清单直接替换（大文件分段处理）
fn apply_items_large(
    content: &str,
    items: &[crate::models::session::DesensitizeItem],
) -> Result<(String, Vec<(String, String, String)>), String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut result_parts: Vec<String> = Vec::new();
    let mut all_mappings: Vec<(String, String, String)> = Vec::new();
    let mut global_entity_counters: HashMap<String, usize> = HashMap::new();
    let mut global_value_to_placeholder: HashMap<String, String> = HashMap::new();

    let sorted_items: Vec<_> = items.iter().collect();

    let mut chunk_lines: Vec<&str> = Vec::new();
    let mut chunk_size = 0;

    let process_chunk = |chunk: &Vec<&str>,
                         counters: &mut HashMap<String, usize>,
                         v2p: &mut HashMap<String, String>,
                         all_m: &mut Vec<(String, String, String)>|
     -> Result<String, String> {
        let chunk_str = chunk.join("\n");
        // 过滤出在本 chunk 中实际出现的 items（快速路径）
        let relevant: Vec<crate::models::session::DesensitizeItem> = sorted_items
            .iter()
            .filter(|item| !item.original_value.is_empty() && chunk_str.contains(item.original_value.as_str()))
            .map(|item| (*item).clone())
            .collect();
        let chunk_result = replace_by_position(&chunk_str, &relevant, counters, v2p, all_m);
        Ok(chunk_result)
    };

    for line in lines {
        chunk_lines.push(line);
        chunk_size += line.len() + 1;
        if chunk_size >= CHUNK_THRESHOLD {
            let part = process_chunk(
                &chunk_lines,
                &mut global_entity_counters,
                &mut global_value_to_placeholder,
                &mut all_mappings,
            )?;
            result_parts.push(part);
            chunk_lines.clear();
            chunk_size = 0;
        }
    }
    if !chunk_lines.is_empty() {
        let part = process_chunk(
            &chunk_lines,
            &mut global_entity_counters,
            &mut global_value_to_placeholder,
            &mut all_mappings,
        )?;
        result_parts.push(part);
    }

    Ok((result_parts.join("\n"), all_mappings))
}

fn get_enabled_entities() -> Result<Vec<Entity>, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, name, entity_type, synonyms, regex_pattern, strategy, enabled, created_at, updated_at 
         FROM sensitive_entities WHERE enabled = 1"
    ).map_err(|e| e.to_string())?;
    
    let entities = stmt.query_map([], |row| {
        let entity_type_str: String = row.get(2)?;
        let strategy_str: String = row.get(5)?;
        let synonyms_str: String = row.get(3)?;
        
        Ok(Entity {
            id: row.get(0)?,
            name: row.get(1)?,
            entity_type: match entity_type_str.as_str() {
                "custom" => crate::models::entity::EntityType::Custom,
                _ => crate::models::entity::EntityType::Builtin,
            },
            synonyms: serde_json::from_str(&synonyms_str).unwrap_or_default(),
            regex_pattern: row.get(4)?,
            strategy: match strategy_str.as_str() {
                "empty" => Strategy::Empty,
                _ => Strategy::RandomReplace,
            },
            enabled: true,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }).map_err(|e| e.to_string())?;
    
    entities.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// 判断 NER 识别的 LOC 实体是否实际上是工程/法律术语（误识别过滤）
fn is_non_location_phrase(text: &str) -> bool {
    const NON_LOC_KEYWORDS: &[&str] = &[
        "线路", "电缆", "工程", "争端", "仲裁", "合同", "协议", "条款",
        "任何", "部分", "本体", "管道", "设备", "装置", "系统", "线段",
        "施工", "安装", "建设", "改造",
        "路费", "路上", "路过", "路途", "路程",
    ];
    NON_LOC_KEYWORDS.iter().any(|kw| text.contains(kw))
}

/// 判断经过裁剪的组织名称是否有效（过滤虚词/代词/连词开头的误识别）
fn is_valid_org_name(name: &str) -> bool {
    // 至少 4 个字符（2 字前缀 + 2 字后缀），避免"银行"/"公司"单独命中
    if name.chars().count() < 4 {
        return false;
    }
    // 包含动词完成助词"了"，说明是动词短语而非机构名
    if name.contains('了') {
        return false;
    }
    // 以下虚词/代词/连词/介词开头时，通常不是真实机构名
    const INVALID_STARTERS: &[&str] = &[
        "如果", "若该", "若",
        "该", "此", "那", "这", "其",
        "包括", "承包", "应由", "认可",
        "任何", "是", "或", "等", "及",
        "注册", "保证", "满足",
        // 句子连接词/副词（合同语体常见句首词）
        "当", "但", "则", "所", "且", "而", "虽", "若",
        "并", "均", "应", "须", "可", "凡",
        // 数量短语
        "一份", "一个", "一种", "一项", "一笔", "一批",
        // 描述性定语（不会是机构名开头）
        "无条件", "有条件", "不可撤销", "可撤销",
        "指定", "约定", "规定", "相应", "延长",
        // 银行/医院/学校等字段标签词
        "开户", "收款", "付款", "汇款", "转账", "结算",
        "就读", "毕业", "入职", "离职", "就诊", "住院",
    ];
    if INVALID_STARTERS.iter().any(|s| name.starts_with(s)) {
        return false;
    }
    true
}

/// 二阶段收缩组织名称左边界
/// - 阶段1：找最近的地名作为开头（有地名就从地名开始）
/// - 阶段2：降级到找最后一个分隔符的位置（空格、标点、括号等）
fn trim_org_name(raw: &str) -> String {
    // 常见地名前缀列表（省、市、直辖市、自治区）
    const PLACE_NAMES: &[&str] = &[
        "北京", "上海", "天津", "重庆",
        "广州", "深圳", "杭州", "南京", "武汉", "成都", "西安", "苏州",
        "郑州", "长沙", "宁波", "青岛", "合肥", "厦门", "福州", "济南",
        "东莞", "佛山", "无锡", "南宁", "昆明", "哈尔滨", "沈阳", "长春",
        "大连", "贵阳", "太原", "石家庄", "南昌", "兰州", "银川",
        "呼和浩特", "乌鲁木齐", "拉萨", "西宁", "海口", "三亚",
        "山东", "山西", "湖南", "湖北", "河南", "河北", "云南", "贵州",
        "四川", "广东", "浙江", "江苏", "江西", "安徽", "福建", "广西",
        "陕西", "甘肃", "青海", "内蒙古", "黑龙江", "辽宁", "吉林",
        "新疆", "西藏", "宁夏", "海南",
    ];

    // 分隔符判断：用 codepoint 数値避免 char 字面量编码问题
    let is_delim = |ch: char| -> bool {
        let cp = ch as u32;
        // ASCII 标点/空白
        matches!(ch, ' ' | '\t' | ',' | ':' | ';' | '(' | ')')
        // CJK 标点
        || cp == 0xFF0C // ，
        || cp == 0x3002 // 。
        || cp == 0xFF01 // ！
        || cp == 0xFF1F // ？
        || cp == 0x3001 // 、
        || cp == 0x300C // 「
        || cp == 0x300D // 」
        || cp == 0x3010 // 【
        || cp == 0x3011 // 】
        || cp == 0xFF08 // （
        || cp == 0xFF09 // ）
        || cp == 0x201C // “
        || cp == 0x201D // ”
        || cp == 0x2018 // ‘
        || cp == 0x2019 // ’
        || cp == 0xFF1A // ：
        || cp == 0xFF1B // ；
        // 介词/连词边界
        || cp == 0x4EE5 // 以
        || cp == 0x6309 // 按
        || cp == 0x6216 // 或
        || cp == 0x4E0E // 与
        || cp == 0x4ECE // 从
        || cp == 0x5411 // 向
        // 结构助词"的"：描述性定语之后通常是通名而非专名
        || cp == 0x7684 // 的
    };


    // 阶段1：找 raw 中最早出现的地名位置
    let mut place_start: Option<usize> = None;
    for place in PLACE_NAMES {
        if let Some(pos) = raw.find(place) {
            place_start = Some(match place_start {
                Some(cur) => cur.min(pos),
                None => pos,
            });
        }
    }
    if let Some(pos) = place_start {
        return raw[pos..].to_string();
    }

    // 阶段2：找最后一个分隔符，从其后面开始
    // 倒序遍历字符，找到第一个分隔符（也就是最后一个）
    let mut last_delim_end: Option<usize> = None;
    let mut byte_pos = 0;
    for ch in raw.chars() {
        if is_delim(ch) {
            last_delim_end = Some(byte_pos + ch.len_utf8());
        }
        byte_pos += ch.len_utf8();
    }
    if let Some(pos) = last_delim_end {
        let trimmed = &raw[pos..];
        if trimmed.chars().count() >= 2 {
            return trimmed.to_string();
        }
    }

    // 阶段3：没有分隔符，返回原始匹配（可能本身就是很干净的组织名）
    raw.to_string()
}

fn scan_content(content: &str, entities: &[Entity]) -> Result<Vec<MatchInfo>, String> {
    let mut all_matches: Vec<MatchInfo> = Vec::new();
    let content_bytes = content.as_bytes();
    let content_chars: Vec<char> = content.chars().collect();
    // 辅助函数：通过字节偏移获得前一个字符
    let char_before = |byte_pos: usize| -> Option<char> {
        if byte_pos == 0 {
            return None;
        }
        // 倒序扫描 UTF-8 字符边界
        let s = &content[..byte_pos];
        s.chars().last()
    };
    let char_after = |byte_end: usize| -> Option<char> {
        if byte_end >= content.len() {
            return None;
        }
        content[byte_end..].chars().next()
    };
    let _ = (content_bytes, content_chars); // 暂不直接使用
    
    for entity in entities {
        // 使用正则表达式匹配（使用缓存）
        if let Some(pattern) = &entity.regex_pattern {
            if !pattern.is_empty() {
                if let Some(re) = get_or_compile_regex(pattern) {
                    for mat in re.find_iter(content) {
                        // 对 IP 地址正则：检查前后字符，避免将章节号识别为 IP
                        if entity.name == "IP地址" {
                            let before = char_before(mat.start());
                            let after = char_after(mat.end());
                            let is_extended_number = |c: Option<char>| -> bool {
                                matches!(c, Some(d) if d.is_ascii_digit() || d == '.')
                            };
                            if is_extended_number(before) || is_extended_number(after) {
                                continue;
                            }
                            let is_text_char = |c: Option<char>| -> bool {
                                matches!(c, Some(ch) if ch.is_alphabetic())
                            };
                            if is_text_char(before) || is_text_char(after) {
                                continue;
                            }
                            // 后面紧跟顿号（、）时，大概率是章节序号列举
                            if matches!(after, Some('、')) {
                                continue;
                            }
                            // 前面是行首/空白，后面是中文标点，也视为章节序号
                            let before_is_boundary = matches!(before, None | Some(' ') | Some('\t') | Some('\n') | Some('\r'));
                            let after_is_chinese_punct = matches!(after, Some('，') | Some('。') | Some('：') | Some('；'));
                            if before_is_boundary && after_is_chinese_punct {
                                continue;
                            }
                            // 前方最近的非空白字符是换行或不存在（= 位于行首），大概率是章节序号
                            let preceding_non_ws = content[..mat.start()]
                                .chars()
                                .rev()
                                .find(|c| !matches!(c, ' ' | '\t'));
                            if matches!(preceding_non_ws, None | Some('\n') | Some('\r')) {
                                continue;
                            }
                        }

                        // 对组织类实体：二阶段边界收缩
                        // 正则已捕获到以机构词结尾的片段，但左边界可能包含无关词
                        // 阶段1：找到最近的地名作为开头
                        // 阶段2：降级到空格、标点、括号等作为分隔
                        let final_value = if entity.name == "公司/组织名称"
                            || entity.name == "地名机构"
                        {
                            let raw = mat.as_str();
                            let trimmed = trim_org_name(raw);
                            // 过滤以虚词/代词/连词开头的误识别（如"如果该银行"、"包括公司"）
                            if is_valid_org_name(&trimmed) {
                                trimmed
                            } else {
                                String::new()
                            }
                        } else {
                            mat.as_str().to_string()
                        };

                        if final_value.is_empty() {
                            continue;
                        }

                        all_matches.push(MatchInfo {
                            start: mat.start(),
                            end: mat.end(),
                            value: final_value,
                            entity_id: entity.id.clone(),
                            entity_name: entity.name.clone(),
                        });
                    }
                }
            }
        }
        
        // 使用同义词匹配
        for synonym in &entity.synonyms {
            for (idx, _) in content.match_indices(synonym.as_str()) {
                all_matches.push(MatchInfo {
                    start: idx,
                    end: idx + synonym.len(),
                    value: synonym.to_string(),
                    entity_id: entity.id.clone(),
                    entity_name: entity.name.clone(),
                });
            }
        }
    }

    // NER 识别：补充 PER(姓名)、ORG(公司/组织)、LOC(地址)
    // 如果 NER 模型未加载，返回空列表（graceful degradation）
    let ner_spans = ner_scan(content);
    // 构建实体名称 -> id 映射，用于为 NER 结果匹配对应实体
    let entity_name_to_id: HashMap<&str, &str> = entities
        .iter()
        .map(|e| (e.name.as_str(), e.id.as_str()))
        .collect();
    for span in &ner_spans {
        // 过滤 NER LOC 中的工程/法律术语误识别
        if span.label == "LOC" && is_non_location_phrase(&span.text) {
            continue;
        }
        if let Some(entity_name) = label_to_entity_name(&span.label, &span.text) {
            // 只有对应实体已启用时才加入
            if let Some(&entity_id) = entity_name_to_id.get(entity_name) {
                all_matches.push(MatchInfo {
                    start: span.start,
                    end: span.end,
                    value: span.text.clone(),
                    entity_id: entity_id.to_string(),
                    entity_name: entity_name.to_string(),
                });
            }
        }
    }

    // 按位置排序，并去除重叠的匹配（优先保留长的）
    all_matches.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| b.end.cmp(&a.end)));
    
    let mut filtered: Vec<MatchInfo> = Vec::new();
    let mut last_end = 0;
    
    for m in all_matches {
        if m.start >= last_end {
            last_end = m.end;
            filtered.push(m);
        }
    }
    
    Ok(filtered)
}


fn save_session(
    original: &str,
    desensitized: &str,
    mappings: &[(String, String, String)],
) -> Result<String, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Local::now();
    let name = now.format("%Y-%m-%d %H:%M").to_string();
    let created_at = now.format("%Y-%m-%d %H:%M:%S").to_string();

    conn.execute(
        "INSERT INTO sessions (id, name, original_content, desensitized_content, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?5)",
        [&session_id, &name, original, desensitized, &created_at],
    ).map_err(|e| e.to_string())?;

    for (placeholder, original_value, entity_name) in mappings {
        let mapping_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO desensitize_mappings (id, session_id, placeholder, original_value, entity_id, entity_name, created_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)",
            [&mapping_id, &session_id, placeholder, original_value, entity_name, &created_at],
        ).map_err(|e| e.to_string())?;
    }

    Ok(session_id)
}

/// 删除指定会话（重新脱敏时覆盖旧会话）
fn delete_session_by_id(session_id: &str) -> Result<(), String> {
    let conn = get_connection().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM sessions WHERE id = ?1", [session_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
