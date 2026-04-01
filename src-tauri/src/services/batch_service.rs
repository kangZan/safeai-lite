use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::db::get_connection;
use crate::models::batch::{
    BatchExportInput, BatchExecuteInput, BatchExecuteResult, BatchFileResult,
    BatchFileScanStatus, BatchMergedItem, BatchScanInput, BatchScanResult,
    BatchSessionListItem, FolderFileEntry,
};
use crate::services::desensitize_service::replace_by_position;
use crate::services::export_service;
use crate::services::file_service::{self, FileType};
use crate::services::restore_service::perform_restore;

// ── 文件类型白名单 ────────────────────────────────────────────

const SUPPORTED_EXTENSIONS: &[&str] = &["doc", "docx", "xls", "xlsx", "pdf", "txt", "log"];

fn is_supported_extension(ext: &str) -> bool {
    SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str())
}

fn file_type_str(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .filter(|e| is_supported_extension(e))
}

// ── 后端扫描结果缓存（文件内容不经过 IPC 传输）─────────────────

struct CachedBatchFile {
    #[allow(dead_code)]
    path: String,
    filename: String,
    relative_path: String,
    file_type: String,
    content: String,
    status: String,
    error_msg: Option<String>,
}

static SCAN_CACHE: Lazy<Mutex<HashMap<String, Vec<CachedBatchFile>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// ── 文件夹扫描 ────────────────────────────────────────────────

/// 递归扫描文件夹，只返回支持格式的文件条目（不读取内容）。
/// 使用迭代 BFS 避免深递归栈溢出。
pub fn scan_folder(dir: &str) -> Result<Vec<FolderFileEntry>, String> {
    let base = Path::new(dir);
    let mut entries: Vec<FolderFileEntry> = Vec::new();
    let mut dirs_to_visit: std::collections::VecDeque<std::path::PathBuf> =
        std::collections::VecDeque::new();
    dirs_to_visit.push_back(base.to_path_buf());

    while let Some(current) = dirs_to_visit.pop_front() {
        let read_result = std::fs::read_dir(&current)
            .map_err(|e| format!("无法读取目录 {}: {}", current.display(), e))?;

        for entry in read_result {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_dir() {
                dirs_to_visit.push_back(path);
            } else if let Some(ft) = file_type_str(&path) {
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                let relative_path = path
                    .strip_prefix(base)
                    .map(|p| {
                        // 统一路径分隔符为正斜杠（便于前端展示）
                        p.to_string_lossy().replace('\\', "/")
                    })
                    .unwrap_or_else(|_| filename.clone());

                entries.push(FolderFileEntry {
                    path: path.to_string_lossy().to_string(),
                    filename,
                    relative_path,
                    file_type: ft,
                });
            }
        }
    }

    // 按相对路径字母序排列，输出稳定
    entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(entries)
}

// ── 批量扫描 ──────────────────────────────────────────────────

pub fn batch_scan(input: BatchScanInput) -> Result<BatchScanResult, String> {
    let mut cached_files: Vec<CachedBatchFile> = Vec::new();
    let mut file_statuses: Vec<BatchFileScanStatus> = Vec::new();

    for path_str in &input.file_paths {
        let path = Path::new(path_str);
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let file_type = file_type_str(path).unwrap_or_else(|| "unknown".to_string());

        // relative_path 此处退化为文件名（直接多选时无文件夹基准）
        let relative_path = filename.clone();

        match file_service::read_file(path_str) {
            Ok(fc) => {
                // FR-06：检测图片型 PDF（解析后内容为空）
                if fc.file_type == FileType::Pdf && fc.content.trim().is_empty() {
                    let msg = "此 PDF 为扫描件（图片型），文字识别功能尚在开发中".to_string();
                    file_statuses.push(BatchFileScanStatus {
                        path: path_str.clone(),
                        filename: filename.clone(),
                        relative_path: relative_path.clone(),
                        file_type: file_type.clone(),
                        status: "failed".to_string(),
                        error_msg: Some(msg.clone()),
                        char_count: 0,
                    });
                    cached_files.push(CachedBatchFile {
                        path: path_str.clone(),
                        filename,
                        relative_path,
                        file_type,
                        content: String::new(),
                        status: "failed".to_string(),
                        error_msg: Some(msg),
                    });
                    continue;
                }

                let char_count = fc.content.chars().count();

                file_statuses.push(BatchFileScanStatus {
                    path: path_str.clone(),
                    filename: filename.clone(),
                    relative_path: relative_path.clone(),
                    file_type: file_type.clone(),
                    status: "success".to_string(),
                    error_msg: None,
                    char_count,
                });
                cached_files.push(CachedBatchFile {
                    path: path_str.clone(),
                    filename,
                    relative_path,
                    file_type,
                    content: fc.content,
                    status: "success".to_string(),
                    error_msg: None,
                });
            }
            Err(e) => {
                let msg = e.to_string();
                file_statuses.push(BatchFileScanStatus {
                    path: path_str.clone(),
                    filename: filename.clone(),
                    relative_path: relative_path.clone(),
                    file_type: file_type.clone(),
                    status: "failed".to_string(),
                    error_msg: Some(msg.clone()),
                    char_count: 0,
                });
                cached_files.push(CachedBatchFile {
                    path: path_str.clone(),
                    filename,
                    relative_path,
                    file_type,
                    content: String::new(),
                    status: "failed".to_string(),
                    error_msg: Some(msg),
                });
            }
        }
    }

    // 逐文件独立扫描，合并时记录每个词来自哪些文件
    // key: original_value → (BatchMergedItem, insertion_order)
    let mut value_to_item: HashMap<String, BatchMergedItem> = HashMap::new();
    let mut insertion_order: Vec<String> = Vec::new();

    for cf in &cached_files {
        if cf.status == "failed" || cf.content.is_empty() {
            continue;
        }
        let scan_input = crate::models::session::ScanInput {
            content: cf.content.clone(),
        };
        if let Ok(result) = crate::services::desensitize_service::scan(scan_input) {
            for item in result.items {
                let entry = value_to_item
                    .entry(item.original_value.clone())
                    .or_insert_with(|| {
                        insertion_order.push(item.original_value.clone());
                        BatchMergedItem {
                            original_value: item.original_value.clone(),
                            entity_name: item.entity_name.clone(),
                            strategy: item.strategy.clone(),
                            source_files: Vec::new(),
                        }
                    });
                // 同一文件只记录一次
                if !entry.source_files.contains(&cf.filename) {
                    entry.source_files.push(cf.filename.clone());
                }
            }
        }
    }

    // 按首次出现顺序排列，保证结果稳定
    let merged_items: Vec<BatchMergedItem> = insertion_order
        .into_iter()
        .filter_map(|v| value_to_item.remove(&v))
        .collect();

    // 存入缓存
    let scan_id = uuid::Uuid::new_v4().to_string();
    {
        let mut cache = SCAN_CACHE.lock().unwrap();
        // 清理过旧缓存（超过 20 批次时，清最早的）
        if cache.len() >= 20 {
            if let Some(oldest_key) = cache.keys().next().cloned() {
                cache.remove(&oldest_key);
            }
        }
        cache.insert(scan_id.clone(), cached_files);
    }

    Ok(BatchScanResult {
        scan_id,
        files: file_statuses,
        merged_items,
    })
}

// ── 批量执行脱敏 ──────────────────────────────────────────────

pub fn batch_execute(input: BatchExecuteInput) -> Result<BatchExecuteResult, String> {
    // 从缓存取回文件内容
    let cached_files: Vec<CachedBatchFile> = {
        let mut cache = SCAN_CACHE.lock().unwrap();
        cache
            .remove(&input.scan_id)
            .ok_or_else(|| "扫描缓存已过期，请重新扫描".to_string())?
    };

    let items = &input.items;
    // 全局共享的计数器和映射表（确保同一词在不同文件中使用相同占位符）
    let mut entity_counters: HashMap<String, usize> = HashMap::new();
    let mut value_to_placeholder: HashMap<String, String> = HashMap::new();
    let mut all_mappings: Vec<(String, String, String)> = Vec::new();

    let mut file_results: Vec<BatchFileResult> = Vec::new();
    let mut desensitized_contents: Vec<String> = Vec::new(); // 与 file_results 同序

    for cf in &cached_files {
        if cf.status == "failed" {
            file_results.push(BatchFileResult {
                filename: cf.filename.clone(),
                relative_path: cf.relative_path.clone(),
                file_type: cf.file_type.clone(),
                status: "failed".to_string(),
                error_msg: cf.error_msg.clone(),
            });
            desensitized_contents.push(String::new());
            continue;
        }

        // 使用共享计数器逐文件替换
        let desensitized = replace_by_position(
            &cf.content,
            items,
            &mut entity_counters,
            &mut value_to_placeholder,
            &mut all_mappings,
        );

        file_results.push(BatchFileResult {
            filename: cf.filename.clone(),
            relative_path: cf.relative_path.clone(),
            file_type: cf.file_type.clone(),
            status: "success".to_string(),
            error_msg: None,
        });
        desensitized_contents.push(desensitized);
    }

    // 去重 all_mappings（同词跨文件可能重复入列）
    let mut seen_ph: std::collections::HashSet<String> = std::collections::HashSet::new();
    let unique_mappings: Vec<_> = all_mappings
        .into_iter()
        .filter(|(ph, _, _)| seen_ph.insert(ph.clone()))
        .collect();

    // 写入数据库
    let success_count = file_results.iter().filter(|r| r.status == "success").count();
    let batch_session_id = save_batch_session(
        &file_results,
        &desensitized_contents,
        &unique_mappings,
        success_count,
    )?;

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    Ok(BatchExecuteResult {
        batch_session_id,
        file_count: file_results.len(),
        success_count,
        mapping_count: unique_mappings.len(),
        files: file_results,
        created_at: now,
    })
}

fn save_batch_session(
    file_results: &[BatchFileResult],
    desensitized_contents: &[String],
    mappings: &[(String, String, String)],
    success_count: usize,
) -> Result<String, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Local::now();
    let name = now.format("批量_%Y-%m-%d %H:%M").to_string();
    let created_at = now.format("%Y-%m-%d %H:%M:%S").to_string();

    conn.execute(
        "INSERT INTO batch_sessions (id, name, mapping_count, file_count, success_count, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        rusqlite::params![
            &session_id,
            &name,
            mappings.len() as i64,
            file_results.len() as i64,
            success_count as i64,
            &created_at,
        ],
    )
    .map_err(|e| e.to_string())?;

    for ((result, content), _) in file_results
        .iter()
        .zip(desensitized_contents.iter())
        .zip(std::iter::repeat(()))
    {
        let file_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO batch_files (id, batch_session_id, filename, relative_path, file_type, desensitized_content, status, error_msg, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &file_id,
                &session_id,
                &result.filename,
                &result.relative_path,
                &result.file_type,
                content,
                &result.status,
                &result.error_msg,
                &created_at,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for (placeholder, original_value, entity_name) in mappings {
        let mapping_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO batch_mappings (id, batch_session_id, placeholder, original_value, entity_name, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                &mapping_id,
                &session_id,
                placeholder,
                original_value,
                entity_name,
                &created_at,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(session_id)
}

// ── 批量导出 ──────────────────────────────────────────────────

pub fn batch_export(input: BatchExportInput) -> Result<String, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;

    // 查询所有成功文件的脱敏内容
    let mut stmt = conn
        .prepare(
            "SELECT filename, relative_path, file_type, desensitized_content
             FROM batch_files
             WHERE batch_session_id = ?1 AND status = 'success'",
        )
        .map_err(|e| e.to_string())?;

    struct ExportFile {
        filename: String,
        relative_path: String,
        file_type: String,
        desensitized_content: String,
    }

    let files: Vec<ExportFile> = stmt
        .query_map(rusqlite::params![&input.batch_session_id], |row| {
            Ok(ExportFile {
                filename: row.get(0)?,
                relative_path: row.get(1)?,
                file_type: row.get(2)?,
                desensitized_content: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let output_dir = Path::new(&input.output_dir);

    for file in &files {
        // 构建输出路径，保留子文件夹结构
        let rel = Path::new(&file.relative_path);
        let stem = rel
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&file.filename);
        let ext = &file.file_type;
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let out_filename = format!("{}_{}_{}.{}", stem, timestamp, "半脱敏版", ext);

        let out_path = if let Some(parent) = rel.parent() {
            if parent.as_os_str().is_empty() {
                output_dir.join(&out_filename)
            } else {
                let dir = output_dir.join(parent);
                std::fs::create_dir_all(&dir).ok();
                dir.join(&out_filename)
            }
        } else {
            output_dir.join(&out_filename)
        };

        // 委托给 export_service
        export_service::export_file(export_service::ExportInput {
            content: file.desensitized_content.clone(),
            format: file.file_type.clone(),
            path: out_path.to_string_lossy().to_string(),
        })
        .map_err(|e| format!("导出 {} 失败: {}", file.filename, e))?;
    }

    if input.zip {
        // 打包为 ZIP
        let zip_path = format!(
            "{}.zip",
            input.output_dir.trim_end_matches(['/', '\\'])
        );
        create_zip(output_dir, &zip_path)?;
        return Ok(zip_path);
    }

    Ok(input.output_dir.clone())
}

fn create_zip(src_dir: &Path, zip_path: &str) -> Result<(), String> {
    use std::io::Write;
    let zip_file = std::fs::File::create(zip_path)
        .map_err(|e| format!("创建 ZIP 文件失败: {}", e))?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let src_dir_str = src_dir.to_string_lossy().to_string();

    // 迭代目录写入 ZIP
    let mut dirs_to_visit: std::collections::VecDeque<std::path::PathBuf> =
        std::collections::VecDeque::new();
    dirs_to_visit.push_back(src_dir.to_path_buf());

    while let Some(current) = dirs_to_visit.pop_front() {
        let entries = std::fs::read_dir(&current)
            .map_err(|e| format!("读取目录失败: {}", e))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                dirs_to_visit.push_back(path);
            } else {
                let rel = path
                    .strip_prefix(src_dir)
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| path.file_name().unwrap_or_default().to_string_lossy().to_string());

                zip.start_file(&rel, options)
                    .map_err(|e| format!("ZIP 写入失败: {}", e))?;
                let data = std::fs::read(&path)
                    .map_err(|e| format!("读取文件失败 {}: {}", src_dir_str, e))?;
                zip.write_all(&data)
                    .map_err(|e| format!("ZIP 写入数据失败: {}", e))?;
            }
        }
    }

    zip.finish().map_err(|e| format!("ZIP 完成失败: {}", e))?;
    Ok(())
}

// ── 批量还原 ──────────────────────────────────────────────────

pub fn batch_restore(batch_session_id: &str) -> Result<Vec<BatchFileResult>, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;

    // 加载统一映射表
    let mut stmt = conn
        .prepare(
            "SELECT placeholder, original_value FROM batch_mappings WHERE batch_session_id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let mappings: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![batch_session_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 查询所有文件（含失败的）
    let mut stmt2 = conn
        .prepare(
            "SELECT filename, relative_path, file_type, desensitized_content, status, error_msg
             FROM batch_files WHERE batch_session_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, String, String, String, String, Option<String>)> = stmt2
        .query_map(rusqlite::params![batch_session_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for (filename, relative_path, file_type, desensitized_content, status, error_msg) in rows {
        if status == "failed" {
            results.push(BatchFileResult {
                filename,
                relative_path,
                file_type,
                status,
                error_msg,
            });
            continue;
        }
        let restored = perform_restore(&desensitized_content, &mappings)
            .unwrap_or_else(|_| desensitized_content.clone());
        // 将还原内容写回（更新 desensitized_content 字段复用作还原内容返回）
        // 实际上我们通过单独的 restored_content 返回更干净，但为保持结构简单，
        // 使用 error_msg 字段临时携带还原后内容（JSON 字段语义在前端明确）
        results.push(BatchFileResult {
            filename,
            relative_path,
            file_type,
            status: "restored".to_string(),
            error_msg: Some(restored), // 前端约定：status=restored 时 error_msg 为还原内容
        });
    }

    Ok(results)
}

// ── 会话列表管理 ──────────────────────────────────────────────

pub fn batch_session_get_all() -> Result<Vec<BatchSessionListItem>, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, file_count, success_count, mapping_count, created_at
             FROM batch_sessions ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let items = stmt
        .query_map([], |row| {
            Ok(BatchSessionListItem {
                id: row.get(0)?,
                name: row.get(1)?,
                file_count: row.get(2)?,
                success_count: row.get(3)?,
                mapping_count: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(items)
}

pub fn batch_session_delete(id: &str) -> Result<bool, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;
    let rows = conn
        .execute(
            "DELETE FROM batch_sessions WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
    Ok(rows > 0)
}
