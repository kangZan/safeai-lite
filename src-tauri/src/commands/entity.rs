use crate::models::entity::Entity;
use crate::services::entity_service::{self, CreateEntityDto, UpdateEntityDto};

#[tauri::command]
pub fn entity_get_all() -> Result<Vec<Entity>, String> {
    entity_service::get_all_entities().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn entity_get_builtin() -> Result<Vec<Entity>, String> {
    entity_service::get_builtin_entities().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn entity_get_custom() -> Result<Vec<Entity>, String> {
    entity_service::get_custom_entities().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn entity_toggle(id: String, enabled: bool) -> Result<bool, String> {
    entity_service::toggle_entity(&id, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn entity_update_strategy(id: String, strategy: String) -> Result<bool, String> {
    entity_service::update_strategy(&id, &strategy)
}

#[tauri::command]
pub fn entity_create(dto: CreateEntityDto) -> Result<Entity, String> {
    entity_service::create_entity(dto)
}

#[tauri::command]
pub fn entity_update(dto: UpdateEntityDto) -> Result<Entity, String> {
    entity_service::update_entity(dto)
}

#[tauri::command]
pub fn entity_update_synonyms(id: String, synonyms: Vec<String>) -> Result<(), String> {
    entity_service::update_entity_synonyms(&id, synonyms)
}

#[tauri::command]
pub fn entity_delete(id: String) -> Result<bool, String> {
    entity_service::delete_entity(&id)
}
