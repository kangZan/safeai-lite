use serde::{Deserialize, Serialize};

// ── 文件夹扫描 ────────────────────────────────────────────────

/// 文件夹递归扫描返回的单文件条目（不读取内容）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderFileEntry {
    pub path: String,
    pub filename: String,
    /// 相对于所选文件夹的路径（含子文件夹前缀）
    pub relative_path: String,
    pub file_type: String,
}

// ── 批量扫描 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchScanInput {
    pub file_paths: Vec<String>,
}

/// 批量扫描中单个识别词条（含来源文件列表）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchMergedItem {
    pub original_value: String,
    pub entity_name: String,
    /// 继承自实体配置的默认策略
    pub strategy: String,
    /// 该词出现在哪些文件中（filename 列表，去重）
    pub source_files: Vec<String>,
}

/// 批量扫描返回结果（文件内容留在后端缓存，不传到前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchScanResult {
    /// 后端缓存 key，execute 时凭此取回文件内容
    pub scan_id: String,
    pub files: Vec<BatchFileScanStatus>,
    pub merged_items: Vec<BatchMergedItem>,
}

/// 单文件的扫描状态（轻量，不含文件内容）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchFileScanStatus {
    pub path: String,
    pub filename: String,
    pub relative_path: String,
    pub file_type: String,
    pub status: String,          // "success" | "failed"
    pub error_msg: Option<String>,
    pub char_count: usize,       // 提取到的文字字符数，用于界面展示
}

// ── 批量执行 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchExecuteInput {
    /// 与 BatchScanResult.scanId 对应
    pub scan_id: String,
    /// 用户确认后的脱敏清单（已排除 excluded 项）
    pub items: Vec<crate::models::session::DesensitizeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchExecuteResult {
    pub batch_session_id: String,
    pub file_count: usize,
    pub success_count: usize,
    pub mapping_count: usize,
    pub files: Vec<BatchFileResult>,
    pub created_at: String,
}

/// 单文件执行结果（只含状态，内容已写入 DB）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchFileResult {
    pub filename: String,
    pub relative_path: String,
    pub file_type: String,
    pub status: String,
    pub error_msg: Option<String>,
}

// ── 批量导出 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchExportInput {
    pub batch_session_id: String,
    pub output_dir: String,
    pub zip: bool,
}

// ── 历史列表 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchSessionListItem {
    pub id: String,
    pub name: String,
    pub file_count: u32,
    pub success_count: u32,
    pub mapping_count: u32,
    pub created_at: String,
}
