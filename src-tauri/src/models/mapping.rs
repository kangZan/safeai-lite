use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapping {
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub placeholder: String,
    #[serde(rename = "originalValue")]
    pub original_value: String,
    #[serde(rename = "entityId")]
    pub entity_id: Option<String>,
    #[serde(rename = "entityName")]
    pub entity_name: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub start: usize,
    pub end: usize,
    pub value: String,
    pub entity_id: String,
    pub entity_name: String,
}
