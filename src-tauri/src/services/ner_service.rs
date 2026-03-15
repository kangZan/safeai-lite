use ndarray::s;
use once_cell::sync::OnceCell;
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tokenizers::Tokenizer;

/// NER 识别结果 span
#[derive(Debug, Clone)]
pub struct NerSpan {
    pub text: String,
    /// 在原始字符串中的字节起始位置
    pub start: usize,
    /// 在原始字符串中的字节结束位置
    pub end: usize,
    /// 实体类型：PER / LOC / ORG / MISC
    pub label: String,
}

/// 全局单例 NER 模型（延迟初始化，避免重复加载）
static NER_MODEL: OnceCell<Mutex<NerEngine>> = OnceCell::new();

/// NER 模型是否正在加载中
static NER_LOADING: AtomicBool = AtomicBool::new(false);

/// NER 模型加载失败的错误信息（仅在加载失败时设置）
static NER_ERROR: OnceCell<String> = OnceCell::new();

pub struct NerEngine {
    session: Session,
    tokenizer: Tokenizer,
    id2label: HashMap<i64, String>,
}

impl NerEngine {
    fn new(model_path: &Path, tokenizer_path: &Path) -> Result<Self, String> {
        let session = Session::builder()
            .map_err(|e| format!("ort builder error: {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| format!("ort opt level error: {e}"))?
            .with_intra_threads(2)
            .map_err(|e| format!("ort threads error: {e}"))?
            .commit_from_file(model_path)
            .map_err(|e| format!("ort load model error: {e}"))?;

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| format!("tokenizer load error: {e}"))?;

        // Xenova/bert-base-chinese-ner 的标签映射
        // https://huggingface.co/Xenova/bert-base-chinese-ner
        let id2label: HashMap<i64, String> = [
            (0, "O"),
            (1, "B-PER"),
            (2, "I-PER"),
            (3, "B-ORG"),
            (4, "I-ORG"),
            (5, "B-LOC"),
            (6, "I-LOC"),
            (7, "B-MISC"),
            (8, "I-MISC"),
        ]
        .iter()
        .map(|(k, v)| (*k, v.to_string()))
        .collect();

        Ok(Self {
            session,
            tokenizer,
            id2label,
        })
    }

    fn predict(&mut self, text: &str) -> Result<Vec<NerSpan>, String> {
        // 限制输入长度（BERT 最大 512 tokens，保留 [CLS] 和 [SEP]）
        // 对长文本分段处理
        if text.is_empty() {
            return Ok(vec![]);
        }

        // 中文字符按 128 字分段（给 tokenizer 留足余量）
        const CHUNK_CHARS: usize = 128;
        let chars: Vec<char> = text.chars().collect();

        if chars.len() <= CHUNK_CHARS {
            return self.predict_chunk(text, 0);
        }

        // 分段处理，合并结果
        let mut all_spans: Vec<NerSpan> = Vec::new();
        let mut char_offset = 0usize;
        let mut byte_offset = 0usize;

        while char_offset < chars.len() {
            let end_char = (char_offset + CHUNK_CHARS).min(chars.len());
            let chunk: String = chars[char_offset..end_char].iter().collect();
            let spans = self.predict_chunk(&chunk, byte_offset)?;
            all_spans.extend(spans);
            // 计算字节偏移
            byte_offset += chunk.len();
            char_offset = end_char;
        }

        Ok(all_spans)
    }

    fn predict_chunk(&mut self, text: &str, byte_base: usize) -> Result<Vec<NerSpan>, String> {
        // tokenize，开启 offset_mapping
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| format!("tokenize error: {e}"))?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&x| x as i64)
            .collect();
        let token_type_ids: Vec<i64> = encoding
            .get_type_ids()
            .iter()
            .map(|&x| x as i64)
            .collect();

        let seq_len = ids.len();
        if seq_len == 0 {
            return Ok(vec![]);
        }

        // 构造 ndarray 输入 [1, seq_len]
        let input_ids =
            ndarray::Array2::from_shape_vec((1, seq_len), ids).map_err(|e| format!("ndarray error: {e}"))?;
        let attention =
            ndarray::Array2::from_shape_vec((1, seq_len), attention_mask)
                .map_err(|e| format!("ndarray error: {e}"))?;
        let token_types =
            ndarray::Array2::from_shape_vec((1, seq_len), token_type_ids)
                .map_err(|e| format!("ndarray error: {e}"))?;

        // 运行推理
        let input_ids_tensor = Tensor::<i64>::from_array(([1, seq_len], input_ids.into_raw_vec_and_offset().0))
            .map_err(|e| format!("tensor error: {e}"))?;
        let attention_tensor = Tensor::<i64>::from_array(([1, seq_len], attention.into_raw_vec_and_offset().0))
            .map_err(|e| format!("tensor error: {e}"))?;
        let token_types_tensor = Tensor::<i64>::from_array(([1, seq_len], token_types.into_raw_vec_and_offset().0))
            .map_err(|e| format!("tensor error: {e}"))?;

        let outputs = self
            .session
            .run(ort::inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_tensor,
                "token_type_ids" => token_types_tensor
            ])
            .map_err(|e| format!("inference error: {e}"))?;

        // 取 logits: [1, seq_len, num_labels]
        let logits = outputs["logits"]
            .try_extract_array::<f32>()
            .map_err(|e| format!("extract logits error: {e}"))?;

        // argmax 得到每个 token 的标签 id
        let logits_2d = logits.slice(s![0, .., ..]);
        let mut label_ids: Vec<i64> = Vec::with_capacity(seq_len);
        for token_logits in logits_2d.outer_iter() {
            let max_idx = token_logits
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i as i64)
                .unwrap_or(0i64);
            label_ids.push(max_idx);
        }

        // BIO 解码：在 outputs 生命期内提前提取所需数据，避免和 bio_decode 中的共享引用冲突
        let spans = {
            let offsets = encoding.get_offsets().to_vec();
            // 出块前释放 outputs
            drop(outputs);
            self.bio_decode(text, &label_ids, &offsets, byte_base)
        };
        Ok(spans)
    }

    fn bio_decode(
        &self,
        text: &str,
        label_ids: &[i64],
        offsets: &[(usize, usize)],
        byte_base: usize,
    ) -> Vec<NerSpan> {
        let mut spans: Vec<NerSpan> = Vec::new();

        // 当前正在累积的实体
        let mut cur_label: Option<String> = None;
        let mut cur_start: usize = 0;
        let mut cur_end: usize = 0;

        for (i, &label_id) in label_ids.iter().enumerate() {
            let tag = self
                .id2label
                .get(&label_id)
                .map(|s| s.as_str())
                .unwrap_or("O");

            let (tok_start, tok_end) = if i < offsets.len() {
                offsets[i]
            } else {
                (0, 0)
            };

            if tag == "O" || tag == "[CLS]" || tag == "[SEP]" {
                // 结束当前实体
                if let Some(label) = cur_label.take() {
                    if let Some(span_text) = text.get(cur_start..cur_end) {
                        let span_text = span_text.trim().to_string();
                        if !span_text.is_empty() {
                            spans.push(NerSpan {
                                text: span_text,
                                start: byte_base + cur_start,
                                end: byte_base + cur_end,
                                label,
                            });
                        }
                    }
                }
            } else if let Some(stripped) = tag.strip_prefix("B-") {
                // 先结束上一个实体
                if let Some(label) = cur_label.take() {
                    if let Some(span_text) = text.get(cur_start..cur_end) {
                        let span_text = span_text.trim().to_string();
                        if !span_text.is_empty() {
                            spans.push(NerSpan {
                                text: span_text,
                                start: byte_base + cur_start,
                                end: byte_base + cur_end,
                                label,
                            });
                        }
                    }
                }
                // 开始新实体
                cur_label = Some(stripped.to_string());
                cur_start = tok_start;
                cur_end = tok_end;
            } else if let Some(stripped) = tag.strip_prefix("I-") {
                // 延续当前实体（需确保类型一致）
                if cur_label.as_deref() == Some(stripped) {
                    cur_end = tok_end;
                } else {
                    // 类型不一致时，作为新实体开始
                    if let Some(label) = cur_label.take() {
                        if let Some(span_text) = text.get(cur_start..cur_end) {
                            let span_text = span_text.trim().to_string();
                            if !span_text.is_empty() {
                                spans.push(NerSpan {
                                    text: span_text,
                                    start: byte_base + cur_start,
                                    end: byte_base + cur_end,
                                    label,
                                });
                            }
                        }
                    }
                    cur_label = Some(stripped.to_string());
                    cur_start = tok_start;
                    cur_end = tok_end;
                }
            }
        }

        // 收尾最后一个实体
        if let Some(label) = cur_label.take() {
            if let Some(span_text) = text.get(cur_start..cur_end) {
                let span_text = span_text.trim().to_string();
                if !span_text.is_empty() {
                    spans.push(NerSpan {
                        text: span_text,
                        start: byte_base + cur_start,
                        end: byte_base + cur_end,
                        label,
                    });
                }
            }
        }

        spans
    }
}

/// 初始化全局 NER 模型（应在 app 启动时调用一次）
/// dll_path: onnxruntime.dll 所在目录（load-dynamic 模式需要）
pub fn init_ner(model_path: &Path, tokenizer_path: &Path) -> Result<(), String> {
    if NER_MODEL.get().is_some() {
        return Ok(()); // 已初始化
    }

    NER_LOADING.store(true, Ordering::SeqCst);

    let result: Result<(), String> = (|| {
        // load-dynamic 模式：手动指定 onnxruntime.dll 路径
        let dll_path = model_path
            .parent()
            .ok_or("invalid model path")?
            .join("onnxruntime.dll");
        ort::init_from(dll_path)
            .map_err(|e| format!("ort init error: {e}"))?
            .commit();

        let engine = NerEngine::new(model_path, tokenizer_path)?;
        NER_MODEL
            .set(Mutex::new(engine))
            .map_err(|_| "NER model already initialized".to_string())?;
        Ok(())
    })();

    NER_LOADING.store(false, Ordering::SeqCst);

    if let Err(ref e) = result {
        let _ = NER_ERROR.set(e.clone());
    }

    result
}

/// 返回 NER 模型是否已就绪
pub fn ner_is_ready() -> bool {
    NER_MODEL.get().is_some()
}

/// 返回 NER 模型是否正在加载
pub fn ner_is_loading() -> bool {
    NER_LOADING.load(Ordering::SeqCst)
}

/// 返回 NER 加载失败的错误信息（若加载成功则为 None）
pub fn ner_get_error() -> Option<String> {
    NER_ERROR.get().cloned()
}

/// 对文本执行 NER 识别，返回所有 span。
/// 若模型仍在加载中，等待最多 15 秒；若未初始化或加载失败，返回空列表。
pub fn ner_scan(text: &str) -> Vec<NerSpan> {
    // 若模型还在加载，最多等待 15 秒
    if NER_LOADING.load(Ordering::SeqCst) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while NER_LOADING.load(Ordering::SeqCst) {
            if std::time::Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    let Some(mutex) = NER_MODEL.get() else {
        return vec![];
    };
    let mut engine = mutex.lock().unwrap();
    engine.predict(text).unwrap_or_default()
}

/// NER 标签 → 内置实体名称映射
///
/// 对于 ORG 标签，通过文本前缀判断属于"地名机构"还是"公司/组织名称"：
///   - 以省市地名开头（如"北京大学"、"上海市人民医院"）→ 地名机构
///   - 其余商业公司名 → 公司/组织名称
/// MISC（杂项命名实体）也归入地名机构，中文 NER 中 MISC 常见于政府机关、
/// 事业单位等非典型 ORG 实体。
pub fn label_to_entity_name(label: &str, text: &str) -> Option<&'static str> {
    match label {
        "PER" => Some("姓名/用户名"),
        "ORG" => {
            if org_starts_with_place_name(text) {
                Some("地名机构")
            } else {
                Some("公司/组织名称")
            }
        }
        "LOC" => Some("物理地址"),
        "MISC" => Some("地名机构"),
        _ => None,
    }
}

/// 判断 ORG 实体是否以省市地名开头（用于区分"地名机构"与纯商业"公司/组织名称"）
fn org_starts_with_place_name(text: &str) -> bool {
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
        "新疆", "西藏", "宁夏", "海南", "中国", "全国",
    ];
    PLACE_NAMES.iter().any(|p| text.starts_with(p))
}
