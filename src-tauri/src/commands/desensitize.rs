use crate::models::session::{DesensitizeInput, DesensitizeResult, ScanInput, ScanResult};
use crate::services::desensitize_service;
use crate::services::ner_service;

/// 扫描文案中的敏感词（只扫不写库）
#[tauri::command]
pub fn desensitize_scan(input: ScanInput) -> Result<ScanResult, String> {
    desensitize_service::scan(input)
}

/// 按用户确认的清单执行脱敏
#[tauri::command]
pub fn desensitize_execute(input: DesensitizeInput) -> Result<DesensitizeResult, String> {
    desensitize_service::desensitize(input)
}

#[derive(serde::Serialize)]
pub struct NerStatus {
    pub ready: bool,
    pub loading: bool,
    pub error: Option<String>,
}

/// 查询 NER 模型加载状态（用于前端诊断）
#[tauri::command]
pub fn ner_get_status() -> NerStatus {
    NerStatus {
        ready: ner_service::ner_is_ready(),
        loading: ner_service::ner_is_loading(),
        error: ner_service::ner_get_error(),
    }
}
