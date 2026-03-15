use crate::models::session::{RestoreInput, RestoreResult};
use crate::services::restore_service;

#[tauri::command]
pub fn restore_execute(input: RestoreInput) -> Result<RestoreResult, String> {
    restore_service::restore(input)
}
