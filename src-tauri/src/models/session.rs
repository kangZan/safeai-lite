use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    #[serde(rename = "originalContent")]
    pub original_content: String,
    #[serde(rename = "desensitizedContent")]
    pub desensitized_content: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// 会话列表项（精简字段，用于列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListItem {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(rename = "mappingCount")]
    pub mapping_count: u32,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub preview: String,
    #[serde(rename = "desensitizedContent")]
    pub desensitized_content: String,
}

/// 脱敏映射项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingItem {
    pub id: String,
    pub placeholder: String,
    #[serde(rename = "originalValue")]
    pub original_value: String,
    #[serde(rename = "entityName")]
    pub entity_name: String,
}

/// 会话详情（含映射关系，用于加载历史会话）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    pub id: String,
    pub name: String,
    #[serde(rename = "originalContent")]
    pub original_content: String,
    #[serde(rename = "desensitizedContent")]
    pub desensitized_content: String,
    pub mappings: Vec<MappingItem>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// ── 扫描阶段 ──────────────────────────────────────────

/// 扫描输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanInput {
    pub content: String,
}

/// 扫描结果项（去重后，每个原文一条）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultItem {
    #[serde(rename = "originalValue")]
    pub original_value: String,
    #[serde(rename = "entityName")]
    pub entity_name: String,
    /// 继承自实体配置的默认策略
    pub strategy: String,
}

/// 扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub items: Vec<ScanResultItem>,
}

/// ── 脱敏阶段 ──────────────────────────────────────────

/// 用户最终确认的单条脱敏项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesensitizeItem {
    #[serde(rename = "originalValue")]
    pub original_value: String,
    #[serde(rename = "entityName")]
    pub entity_name: String,
    /// "random_replace" | "empty"
    pub strategy: String,
}

/// 脱敏输入（携带用户最终清单，不再查全局实体配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesensitizeInput {
    pub content: String,
    /// 用户本次确认的脱敏清单
    pub items: Vec<DesensitizeItem>,
    /// 若存在则覆盖旧会话，否则新建
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesensitizeResult {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "originalContent")]
    pub original_content: String,
    #[serde(rename = "desensitizedContent")]
    pub desensitized_content: String,
    #[serde(rename = "mappingCount")]
    pub mapping_count: usize,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreInput {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub content: String,
}
