use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityType {
    #[serde(rename = "builtin")]
    Builtin,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Strategy {
    #[serde(rename = "random_replace")]
    RandomReplace,
    #[serde(rename = "empty")]
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: EntityType,
    pub synonyms: Vec<String>,
    #[serde(rename = "regexPattern")]
    pub regex_pattern: Option<String>,
    pub strategy: Strategy,
    pub enabled: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

impl Entity {
    #[allow(dead_code)]
    pub fn new_builtin(
        id: &str,
        name: &str,
        regex_pattern: Option<&str>,
        strategy: Strategy,
    ) -> Self {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            entity_type: EntityType::Builtin,
            synonyms: vec![],
            regex_pattern: regex_pattern.map(|s| s.to_string()),
            strategy,
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
