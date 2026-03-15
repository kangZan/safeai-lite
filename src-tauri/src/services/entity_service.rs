use crate::db::get_connection;
use crate::models::entity::{Entity, EntityType, Strategy};
use rusqlite::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateEntityDto {
    pub name: String,
    pub synonyms: Vec<String>,
    pub regex_pattern: Option<String>,
    pub strategy: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEntityDto {
    pub id: String,
    pub name: String,
    pub synonyms: Vec<String>,
    pub regex_pattern: Option<String>,
    pub strategy: String,
    pub enabled: bool,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct EntityCreated {
    pub id: String,
}

pub fn get_all_entities() -> Result<Vec<Entity>> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, entity_type, synonyms, regex_pattern, strategy, enabled, created_at, updated_at 
         FROM sensitive_entities ORDER BY created_at"
    )?;
    
    let entities = stmt.query_map([], |row| {
        let entity_type_str: String = row.get(2)?;
        let strategy_str: String = row.get(5)?;
        let synonyms_str: String = row.get(3)?;
        
        Ok(Entity {
            id: row.get(0)?,
            name: row.get(1)?,
            entity_type: match entity_type_str.as_str() {
                "custom" => EntityType::Custom,
                _ => EntityType::Builtin,
            },
            synonyms: serde_json::from_str(&synonyms_str).unwrap_or_default(),
            regex_pattern: row.get(4)?,
            strategy: match strategy_str.as_str() {
                "empty" => Strategy::Empty,
                _ => Strategy::RandomReplace,
            },
            enabled: row.get::<_, i32>(6)? != 0,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?;
    
    entities.collect()
}

pub fn get_builtin_entities() -> Result<Vec<Entity>> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, entity_type, synonyms, regex_pattern, strategy, enabled, created_at, updated_at 
         FROM sensitive_entities WHERE entity_type = 'builtin' ORDER BY created_at"
    )?;
    
    let entities = stmt.query_map([], |row| {
        let strategy_str: String = row.get(5)?;
        let synonyms_str: String = row.get(3)?;
        
        Ok(Entity {
            id: row.get(0)?,
            name: row.get(1)?,
            entity_type: EntityType::Builtin,
            synonyms: serde_json::from_str(&synonyms_str).unwrap_or_default(),
            regex_pattern: row.get(4)?,
            strategy: match strategy_str.as_str() {
                "empty" => Strategy::Empty,
                _ => Strategy::RandomReplace,
            },
            enabled: row.get::<_, i32>(6)? != 0,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?;
    
    entities.collect()
}

pub fn toggle_entity(entity_id: &str, enabled: bool) -> Result<bool> {
    let conn = get_connection()?;
    let updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    
    conn.execute(
        "UPDATE sensitive_entities SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
        &[&enabled as &dyn rusqlite::ToSql, &updated_at, &entity_id],
    )?;
    
    Ok(true)
}

pub fn update_strategy(entity_id: &str, strategy: &str) -> Result<bool, String> {
    if strategy != "random_replace" && strategy != "empty" {
        return Err("策略值无效，只支持 random_replace 或 empty".to_string());
    }
    let conn = get_connection().map_err(|e| e.to_string())?;
    let updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    
    conn.execute(
        "UPDATE sensitive_entities SET strategy = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![strategy, &updated_at, entity_id],
    ).map_err(|e| e.to_string())?;
    
    Ok(true)
}

pub fn get_custom_entities() -> Result<Vec<Entity>> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, entity_type, synonyms, regex_pattern, strategy, enabled, created_at, updated_at 
         FROM sensitive_entities WHERE entity_type = 'custom' ORDER BY created_at"
    )?;
    
    let entities = stmt.query_map([], |row| {
        let strategy_str: String = row.get(5)?;
        let synonyms_str: String = row.get(3)?;
        
        Ok(Entity {
            id: row.get(0)?,
            name: row.get(1)?,
            entity_type: EntityType::Custom,
            synonyms: serde_json::from_str(&synonyms_str).unwrap_or_default(),
            regex_pattern: row.get(4)?,
            strategy: match strategy_str.as_str() {
                "empty" => Strategy::Empty,
                _ => Strategy::RandomReplace,
            },
            enabled: row.get::<_, i32>(6)? != 0,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?;
    
    entities.collect()
}

pub fn create_entity(dto: CreateEntityDto) -> Result<Entity, String> {
    // 参数校验
    let name = dto.name.trim().to_string();
    if name.is_empty() {
        return Err("实体名称不能为空".to_string());
    }
    if name.len() > 20 {
        return Err("实体名称不能超过20个字符".to_string());
    }
    if dto.strategy != "random_replace" && dto.strategy != "empty" {
        return Err("策略值无效".to_string());
    }
    // 正则校验
    if let Some(ref pattern) = dto.regex_pattern {
        if !pattern.is_empty() {
            regex::Regex::new(pattern).map_err(|e| format!("正则表达式无效: {}", e))?;
        }
    }
    
    let conn = get_connection().map_err(|e| e.to_string())?;
    
    // 名称唯一性校验
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sensitive_entities WHERE name = ?1",
        rusqlite::params![&name],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if exists > 0 {
        return Err(format!("实体名称 '{}' 已存在", name));
    }
    
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let synonyms_json = serde_json::to_string(&dto.synonyms).map_err(|e| e.to_string())?;
    let regex_val = dto.regex_pattern.as_deref().unwrap_or("");
    let enabled_int: i32 = if dto.enabled { 1 } else { 0 };
    
    conn.execute(
        "INSERT INTO sensitive_entities (id, name, entity_type, synonyms, regex_pattern, strategy, enabled, created_at, updated_at)
         VALUES (?1, ?2, 'custom', ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![&id, &name, &synonyms_json, regex_val, &dto.strategy, enabled_int, &now, &now],
    ).map_err(|e| e.to_string())?;
    
    Ok(Entity {
        id,
        name,
        entity_type: EntityType::Custom,
        synonyms: dto.synonyms,
        regex_pattern: dto.regex_pattern.filter(|s| !s.is_empty()),
        strategy: match dto.strategy.as_str() {
            "empty" => Strategy::Empty,
            _ => Strategy::RandomReplace,
        },
        enabled: dto.enabled,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn update_entity(dto: UpdateEntityDto) -> Result<Entity, String> {
    // 参数校验
    let name = dto.name.trim().to_string();
    if name.is_empty() {
        return Err("实体名称不能为空".to_string());
    }
    if name.len() > 20 {
        return Err("实体名称不能超过20个字符".to_string());
    }
    if dto.strategy != "random_replace" && dto.strategy != "empty" {
        return Err("策略值无效".to_string());
    }
    if let Some(ref pattern) = dto.regex_pattern {
        if !pattern.is_empty() {
            regex::Regex::new(pattern).map_err(|e| format!("正则表达式无效: {}", e))?;
        }
    }
    
    let conn = get_connection().map_err(|e| e.to_string())?;
    
    // 名称唯一性校验（排除自身）
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sensitive_entities WHERE name = ?1 AND id != ?2",
        rusqlite::params![&name, &dto.id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if exists > 0 {
        return Err(format!("实体名称 '{}' 已存在", name));
    }
    
    // 只允许修改自定义实体
    let entity_type: String = conn.query_row(
        "SELECT entity_type FROM sensitive_entities WHERE id = ?1",
        rusqlite::params![&dto.id],
        |row| row.get(0),
    ).map_err(|_| "实体不存在".to_string())?;
    if entity_type != "custom" {
        return Err("内置实体不支持修改名称和同义词，请使用策略修改接口".to_string());
    }
    
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let synonyms_json = serde_json::to_string(&dto.synonyms).map_err(|e| e.to_string())?;
    let regex_val = dto.regex_pattern.as_deref().unwrap_or("");
    let enabled_int: i32 = if dto.enabled { 1 } else { 0 };
    
    conn.execute(
        "UPDATE sensitive_entities SET name = ?1, synonyms = ?2, regex_pattern = ?3, strategy = ?4, enabled = ?5, updated_at = ?6 WHERE id = ?7",
        rusqlite::params![&name, &synonyms_json, regex_val, &dto.strategy, enabled_int, &now, &dto.id],
    ).map_err(|e| e.to_string())?;
    
    Ok(Entity {
        id: dto.id,
        name,
        entity_type: EntityType::Custom,
        synonyms: dto.synonyms,
        regex_pattern: dto.regex_pattern.filter(|s| !s.is_empty()),
        strategy: match dto.strategy.as_str() {
            "empty" => Strategy::Empty,
            _ => Strategy::RandomReplace,
        },
        enabled: dto.enabled,
        created_at: String::new(),
        updated_at: now,
    })
}

/// 更新实体的同义词列表（内置和自定义实体均支持）
pub fn update_entity_synonyms(entity_id: &str, synonyms: Vec<String>) -> Result<(), String> {
    let conn = get_connection().map_err(|e| e.to_string())?;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let synonyms_json = serde_json::to_string(&synonyms).map_err(|e| e.to_string())?;
    let rows = conn.execute(
        "UPDATE sensitive_entities SET synonyms = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![&synonyms_json, &now, entity_id],
    ).map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err("实体不存在".to_string());
    }
    Ok(())
}

pub fn delete_entity(entity_id: &str) -> Result<bool, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;
    
    // 只允许删除自定义实体
    let entity_type: String = conn.query_row(
        "SELECT entity_type FROM sensitive_entities WHERE id = ?1",
        rusqlite::params![entity_id],
        |row| row.get(0),
    ).map_err(|_| "实体不存在".to_string())?;
    if entity_type != "custom" {
        return Err("内置实体不能删除".to_string());
    }
    
    conn.execute(
        "DELETE FROM sensitive_entities WHERE id = ?1",
        rusqlite::params![entity_id],
    ).map_err(|e| e.to_string())?;
    
    Ok(true)
}
