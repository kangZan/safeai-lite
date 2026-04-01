use once_cell::sync::OnceCell;
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

static DB_CONNECTION: OnceCell<Mutex<Connection>> = OnceCell::new();

pub fn init_database() -> Result<()> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;
    
    // 创建表结构
    create_tables(&conn)?;
    
    // 初始化内置实体数据
    init_builtin_entities(&conn)?;
    
    // 迁移：更新已有内置实体的正则
    migrate_builtin_entities(&conn)?;
    
    DB_CONNECTION.set(Mutex::new(conn)).ok();
    Ok(())
}

pub fn get_connection() -> Result<std::sync::MutexGuard<'static, Connection>> {
    DB_CONNECTION
        .get()
        .ok_or_else(|| rusqlite::Error::InvalidPath(PathBuf::from("数据库未初始化")))
        .map(|mutex| mutex.lock().unwrap())
}

fn get_db_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| rusqlite::Error::InvalidPath(PathBuf::from("无法获取用户目录")))?;
    
    let app_dir = home_dir.join("Documents").join("SafeAI-Lite").join("data");
    std::fs::create_dir_all(&app_dir).map_err(|e| rusqlite::Error::InvalidPath(PathBuf::from(e.to_string())))?;
    
    Ok(app_dir.join("safeai.db"))
}

fn create_tables(conn: &Connection) -> Result<()> {
    // 敏感实体表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sensitive_entities (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            entity_type TEXT NOT NULL CHECK(entity_type IN ('builtin', 'custom')),
            synonyms TEXT,
            regex_pattern TEXT,
            strategy TEXT NOT NULL CHECK(strategy IN ('random_replace', 'empty')),
            enabled INTEGER DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // 会话表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            original_content TEXT NOT NULL,
            desensitized_content TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // 脱敏映射表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS desensitize_mappings (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            placeholder TEXT NOT NULL,
            original_value TEXT NOT NULL,
            entity_id TEXT,
            entity_name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (entity_id) REFERENCES sensitive_entities(id) ON DELETE SET NULL
        )",
        [],
    )?;

    // 创建索引（性能优化）
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_mappings_session ON desensitize_mappings(session_id)",
        [],
    )?;

    // 会话表定制化时间索引
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_created ON sessions(created_at DESC)",
        [],
    )?;

    // 实体表类型并包含生效状态索引
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_entities_type_enabled ON sensitive_entities(entity_type, enabled)",
        [],
    )?;

    // ── v0.2.0 批量脱敏表 ──────────────────────────────────────

    // 批量会话主表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS batch_sessions (
            id            TEXT PRIMARY KEY,
            name          TEXT NOT NULL,
            mapping_count INTEGER NOT NULL DEFAULT 0,
            file_count    INTEGER NOT NULL DEFAULT 0,
            success_count INTEGER NOT NULL DEFAULT 0,
            created_at    TEXT NOT NULL,
            updated_at    TEXT NOT NULL
        )",
        [],
    )?;

    // 批量会话下的单文件结果
    conn.execute(
        "CREATE TABLE IF NOT EXISTS batch_files (
            id                   TEXT PRIMARY KEY,
            batch_session_id     TEXT NOT NULL,
            filename             TEXT NOT NULL,
            relative_path        TEXT NOT NULL,
            file_type            TEXT NOT NULL,
            desensitized_content TEXT NOT NULL DEFAULT '',
            status               TEXT NOT NULL DEFAULT 'success'
                                 CHECK(status IN ('success','failed')),
            error_msg            TEXT,
            created_at           TEXT NOT NULL,
            FOREIGN KEY (batch_session_id) REFERENCES batch_sessions(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // 批量脱敏映射表（统一映射，不挂 sessions 表）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS batch_mappings (
            id               TEXT PRIMARY KEY,
            batch_session_id TEXT NOT NULL,
            placeholder      TEXT NOT NULL,
            original_value   TEXT NOT NULL,
            entity_name      TEXT NOT NULL,
            created_at       TEXT NOT NULL,
            FOREIGN KEY (batch_session_id) REFERENCES batch_sessions(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_batch_files_session ON batch_files(batch_session_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_batch_mappings_session ON batch_mappings(batch_session_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_batch_sessions_created ON batch_sessions(created_at DESC)",
        [],
    )?;

    Ok(())
}

fn init_builtin_entities(conn: &Connection) -> Result<()> {
    // 如果已有内置实体，跳过初始化
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sensitive_entities WHERE entity_type = 'builtin'",
        [],
        |row| row.get(0),
    )?;
    if count > 0 {
        return Ok(());
    }

    let builtin_entities: Vec<(&str, Option<&str>)> = vec![
        ("物理地址", Some(r"")),
        ("邮箱地址", Some(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")),
        ("姓名/用户名", None), // 由 NER 识别（PER）
        ("公司/组织名称", None), // 由 NER 识别（ORG）
        ("电话号码", Some(r"1[3-9]\d{9}|0\d{2,3}-\d{7,8}")),
        ("银行账户/卡号", Some(r"\d{16,19}")),
        ("URL网址", Some(r"https?://[^\s]+")),
        ("IP地址", Some(r"(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)\.(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)\.(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)\.(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)")),
        ("地名机构", None), // 由 NER 识别（ORG）
    ];

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    for (idx, (name, pattern)) in builtin_entities.iter().enumerate() {
        let id = format!("builtin_{}", idx);
        conn.execute(
            "INSERT OR IGNORE INTO sensitive_entities (id, name, entity_type, synonyms, regex_pattern, strategy, enabled, created_at, updated_at)
             VALUES (?1, ?2, 'builtin', '[]', ?3, 'random_replace', 1, ?4, ?5)",
            [&id, &name.to_string(), &pattern.unwrap_or("").to_string(), &now, &now],
        )?;
    }

    Ok(())
}

/// 迁移：更新内置实体的正则表达式（修复旧版本数据库中错误的正则）
fn migrate_builtin_entities(conn: &Connection) -> Result<()> {
    // 修复电话号码正则：固话必须带区号前缀（0xx-），防止裸7-8位数字（金额等）误匹配
    let phone_regex = r"1[3-9]\d{9}|0\d{2,3}-\d{7,8}";
    conn.execute(
        "UPDATE sensitive_entities SET regex_pattern = ?1 WHERE name = '电话号码' AND entity_type = 'builtin'",
        [phone_regex],
    )?;

    // 修复 IP 地址的过宽正则（旧版会错误匹配章节号如 1.1.3.2）
    let new_ip_regex = r"(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)\.(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)\.(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)\.(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)";
    conn.execute(
        "UPDATE sensitive_entities SET regex_pattern = ?1 WHERE name = 'IP地址' AND entity_type = 'builtin'",
        [new_ip_regex],
    )?;

    // 公司/组织名称：使用后缀锚定正则作为 NER 的兜底补充
    // 匹配以"公司/集团/大学/医院..."等机构词结尾的名称，NER 负责识别其余情况
    // 去掉"机构"后缀：设备名（弹簧机构、操作机构等）误匹配率过高，NER 可识别真实组织
    let company_regex = r#"[^\s，,。！？、\n\r「」【】（）""]{2,20}(?:公司|集团|有限|股份|企业|工厂|研究院|大学|学院|银行|医院|学校)"#;
    conn.execute(
        "UPDATE sensitive_entities SET regex_pattern = ?1 WHERE name = '公司/组织名称' AND entity_type = 'builtin'",
        [company_regex],
    )?;

    // 物理地址：仅匹配行政级别地名（省/市/区/县/乡/镇等），不匹配路/街等道路名
    // 前缀要求纯汉字，避免数字误匹配
    // 物理地址完全交给 NER 识别，正则误抓率高，清空
    conn.execute(
        "UPDATE sensitive_entities SET regex_pattern = '' WHERE name = '物理地址' AND entity_type = 'builtin'",
        [],
    )?;

    // 地名机构：依赖 NER 识别（ORG+地名前缀、MISC），无需正则
    conn.execute(
        "UPDATE sensitive_entities SET regex_pattern = '' WHERE name = '地名机构' AND entity_type = 'builtin'",
        [],
    )?;

    // 新增地名机构实体（对旧数据库做 INSERT OR IGNORE）
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "INSERT OR IGNORE INTO sensitive_entities (id, name, entity_type, synonyms, regex_pattern, strategy, enabled, created_at, updated_at)
         VALUES ('builtin_8', '地名机构', 'builtin', '[]', '', 'random_replace', 1, ?1, ?2)",
        [&now, &now],
    )?;

    // 姓名/用户名：内置 NER 模型对法律/合同文体人名识别能力有限，默认禁用
    // 用户可在「敏感实体策略」中手动添加同义词后启用
    conn.execute(
        "UPDATE sensitive_entities SET enabled = 0 WHERE name = '姓名/用户名' AND entity_type = 'builtin'",
        [],
    )?;

    Ok(())
}
