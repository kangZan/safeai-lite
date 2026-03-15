use crate::models::session::{SessionDetail, SessionListItem};
use crate::services::session_service;

/// 获取所有会话列表（按时间倒序）
#[tauri::command]
pub fn session_get_all() -> Result<Vec<SessionListItem>, String> {
    session_service::get_all_sessions()
}

/// 获取会话详情（含映射关系）
#[tauri::command]
pub fn session_get_by_id(id: String) -> Result<SessionDetail, String> {
    session_service::get_session_by_id(&id)
}

/// 删除单个会话（级联删除映射）
#[tauri::command]
pub fn session_delete(id: String) -> Result<bool, String> {
    session_service::delete_session(&id)
}

/// 清空所有会话
#[tauri::command]
pub fn session_clear_all() -> Result<bool, String> {
    session_service::clear_all_sessions()
}
