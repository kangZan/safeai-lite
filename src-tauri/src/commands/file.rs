use crate::services::file_service::{self, FileContent};
use crate::services::export_service::{self, ExportInput};

/// 读取并解析文件内容
/// 
/// # 参数
/// - `path`: 文件绝对路径
/// 
/// # 返回值
/// - `FileContent`: 文件内容和元数据
#[tauri::command]
pub fn file_read(path: String) -> Result<FileContent, String> {
    file_service::read_file(&path).map_err(|e| e.to_string())
}

/// 导出文件
/// 
/// # 参数
/// - `input`: 导出参数（内容、格式、保存路径）
/// 
/// # 返回值
/// - `String`: 导出后的文件路径
#[tauri::command]
pub fn file_export(input: ExportInput) -> Result<String, String> {
    export_service::export_file(input).map_err(|e| e.to_string())
}
