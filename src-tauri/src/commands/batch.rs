use crate::models::batch::{
    BatchExportInput, BatchExecuteInput, BatchExecuteResult, BatchFileResult,
    BatchScanInput, BatchScanResult, BatchSessionListItem, FolderFileEntry,
};
use crate::services::batch_service;

/// 递归扫描文件夹，返回支持格式的文件条目（不读取内容）
#[tauri::command]
pub fn batch_scan_folder(dir: String) -> Result<Vec<FolderFileEntry>, String> {
    batch_service::scan_folder(&dir)
}

/// 读取所有文件内容并执行统一扫描，返回合并后的识别结果
#[tauri::command]
pub fn batch_scan(input: BatchScanInput) -> Result<BatchScanResult, String> {
    batch_service::batch_scan(input)
}

/// 对所有文件应用用户确认的脱敏清单，结果写入 DB
#[tauri::command]
pub fn batch_execute(input: BatchExecuteInput) -> Result<BatchExecuteResult, String> {
    batch_service::batch_execute(input)
}

/// 导出批量脱敏结果到指定文件夹（可选打包为 ZIP）
#[tauri::command]
pub fn batch_export(input: BatchExportInput) -> Result<String, String> {
    batch_service::batch_export(input)
}

/// 使用统一映射表对所有文件执行还原
#[tauri::command]
pub fn batch_restore(batch_session_id: String) -> Result<Vec<BatchFileResult>, String> {
    batch_service::batch_restore(&batch_session_id)
}

/// 获取批量会话历史列表
#[tauri::command]
pub fn batch_session_get_all() -> Result<Vec<BatchSessionListItem>, String> {
    batch_service::batch_session_get_all()
}

/// 删除批量会话
#[tauri::command]
pub fn batch_session_delete(id: String) -> Result<bool, String> {
    batch_service::batch_session_delete(&id)
}
