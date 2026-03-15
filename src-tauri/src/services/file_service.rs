use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_FILE_SIZE: u64 = 20 * 1024 * 1024; // 20MB

/// 文件类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Docx,
    Doc,
    Xlsx,
    Xls,
    Pdf,
    Txt,
    Log,
    Unknown,
}

/// 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub size: u64,
    pub pages: Option<u32>,
    pub sheets: Option<Vec<String>>,
}

/// 文件内容结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    pub filename: String,
    pub file_type: FileType,
    pub content: String,
    pub metadata: Option<FileMetadata>,
}

/// 文件错误类型
#[derive(Debug)]
pub enum FileError {
    FileTooLarge,
    UnsupportedFormat(String),
    ParseError(String),
    IoError(String),
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileError::FileTooLarge => write!(f, "文件大小超过20MB限制"),
            FileError::UnsupportedFormat(ext) => write!(f, "不支持的文件格式: {}", ext),
            FileError::ParseError(msg) => write!(f, "文件解析失败: {}", msg),
            FileError::IoError(msg) => write!(f, "文件读取失败: {}", msg),
        }
    }
}

/// 检测文件类型（基于扩展名）
fn detect_file_type(path: &Path) -> FileType {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
        Some("docx") => FileType::Docx,
        Some("doc") => FileType::Doc,
        Some("xlsx") => FileType::Xlsx,
        Some("xls") => FileType::Xls,
        Some("pdf") => FileType::Pdf,
        Some("txt") => FileType::Txt,
        Some("log") => FileType::Log,
        _ => FileType::Unknown,
    }
}

/// 文件魔数表（用于安全校验）
const MAGIC_DOCX: &[u8] = &[0x50, 0x4B, 0x03, 0x04]; // ZIP/DOCX/XLSX
const MAGIC_DOC: &[u8] = &[0xD0, 0xCF, 0x11, 0xE0];  // OLE2/DOC/XLS
const MAGIC_PDF: &[u8] = b"%PDF";                      // PDF

/// 魔数校验：确保文件内容与扩展名匹配
fn validate_file_magic(path: &Path, file_type: &FileType) -> Result<(), FileError> {
    // 只对二进制格式进行魔数校验
    let needs_check = matches!(file_type, FileType::Docx | FileType::Doc | FileType::Xlsx | FileType::Xls | FileType::Pdf);
    if !needs_check {
        return Ok(());
    }

    let mut buf = [0u8; 8];
    let n = {
        use std::io::Read;
        let mut f = std::fs::File::open(path).map_err(|e| FileError::IoError(e.to_string()))?;
        f.read(&mut buf).map_err(|e| FileError::IoError(e.to_string()))?
    };
    let header = &buf[..n];

    let valid = match file_type {
        FileType::Docx | FileType::Xlsx => header.starts_with(MAGIC_DOCX),
        FileType::Doc | FileType::Xls => header.starts_with(MAGIC_DOC) || header.starts_with(MAGIC_DOCX),
        FileType::Pdf => header.starts_with(MAGIC_PDF),
        _ => true,
    };

    if !valid {
        return Err(FileError::ParseError(
            format!("文件内容与扩展名不符，可能是伪造文件或文件损啴，拒绝处理")
        ));
    }

    Ok(())
}

/// 校验文件大小
fn validate_file_size(path: &Path) -> Result<u64, FileError> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| FileError::IoError(e.to_string()))?;
    let size = metadata.len();
    if size > MAX_FILE_SIZE {
        return Err(FileError::FileTooLarge);
    }
    Ok(size)
}

/// 读取文件 - 统一入口
pub fn read_file(path_str: &str) -> Result<FileContent, FileError> {
    let path = Path::new(path_str);
    let file_size = validate_file_size(path)?;
    let file_type = detect_file_type(path);
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // 魔数校验（安全加固）
    validate_file_magic(path, &file_type)?;

    match &file_type {
        FileType::Docx => parse_docx(path, filename, file_size),
        FileType::Doc => parse_doc(path, filename, file_size),
        FileType::Xlsx | FileType::Xls => parse_excel(path, filename, file_size),
        FileType::Pdf => parse_pdf(path, filename, file_size),
        FileType::Txt | FileType::Log => parse_text(path, filename, file_size, file_type),
        FileType::Unknown => {
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();
            Err(FileError::UnsupportedFormat(ext))
        }
    }
}

/// 解析 .docx 文件
fn parse_docx(path: &Path, filename: String, file_size: u64) -> Result<FileContent, FileError> {
    let bytes = std::fs::read(path).map_err(|e| FileError::IoError(e.to_string()))?;
    let docx = docx_rs::read_docx(&bytes)
        .map_err(|e| FileError::ParseError(format!("Word文档解析失败: {:?}", e)))?;

    let mut content = String::new();
    for child in docx.document.children {
        match child {
            docx_rs::DocumentChild::Paragraph(p) => {
                for child in p.children {
                    if let docx_rs::ParagraphChild::Run(run) = child {
                        for child in run.children {
                            if let docx_rs::RunChild::Text(text) = child {
                                content.push_str(&text.text);
                            }
                        }
                    }
                }
                content.push('\n');
            }
            _ => {}
        }
    }

    Ok(FileContent {
        filename,
        file_type: FileType::Docx,
        content,
        metadata: Some(FileMetadata {
            size: file_size,
            pages: None,
            sheets: None,
        }),
    })
}

/// 解析 .doc 文件（旧版 Word，尝试按文本读取）
fn parse_doc(path: &Path, filename: String, file_size: u64) -> Result<FileContent, FileError> {
    // .doc 格式为二进制，尝试提取可读文本
    let bytes = std::fs::read(path).map_err(|e| FileError::IoError(e.to_string()))?;
    
    // 尝试提取可见ASCII/UTF-8字符
    let content = extract_text_from_binary(&bytes);
    
    if content.trim().is_empty() {
        return Err(FileError::ParseError(
            "旧版.doc格式解析受限，建议将文件另存为.docx后重试".to_string()
        ));
    }

    Ok(FileContent {
        filename,
        file_type: FileType::Doc,
        content,
        metadata: Some(FileMetadata {
            size: file_size,
            pages: None,
            sheets: None,
        }),
    })
}

/// 从二进制文件中提取可读文本（用于旧版.doc）
fn extract_text_from_binary(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        // 尝试解析UTF-8序列
        if bytes[i] >= 0x20 && bytes[i] < 0x7F {
            result.push(bytes[i] as char);
        } else if bytes[i] == b'\n' || bytes[i] == b'\r' {
            result.push('\n');
        }
        i += 1;
    }
    
    // 清理多余的连续空行
    let lines: Vec<&str> = result.lines()
        .filter(|l| l.trim().len() > 2)
        .collect();
    lines.join("\n")
}

/// 解析 Excel 文件 (.xlsx / .xls)
fn parse_excel(path: &Path, filename: String, file_size: u64) -> Result<FileContent, FileError> {
    use calamine::{open_workbook_auto, Reader, Data};
    
    let mut workbook = open_workbook_auto(path)
        .map_err(|e| FileError::ParseError(format!("Excel解析失败: {}", e)))?;
    
    let sheet_names = workbook.sheet_names().to_owned();
    let mut content = String::new();
    
    for sheet_name in &sheet_names {
        content.push_str(&format!("## {}\n\n", sheet_name));
        
        match workbook.worksheet_range(sheet_name) {
            Ok(range) => {
                let mut first_row = true;
                for row in range.rows() {
                    let row_text: Vec<String> = row.iter()
                        .map(|cell| match cell {
                            Data::String(s) => s.clone(),
                            Data::Float(f) => {
                                if *f == f.floor() && f.abs() < 1e15 {
                                    format!("{}", *f as i64)
                                } else {
                                    format!("{}", f)
                                }
                            }
                            Data::Int(i) => i.to_string(),
                            Data::Bool(b) => b.to_string(),
                            Data::Empty => String::new(),
                            _ => cell.to_string(),
                        })
                        .collect();
                    
                    let row_str = row_text.join(" | ");
                    content.push_str(&row_str);
                    content.push('\n');
                    
                    // 在第一行后添加表格分隔线（Markdown表格格式）
                    if first_row {
                        let separator: Vec<String> = row_text.iter()
                            .map(|_| "---".to_string())
                            .collect();
                        content.push_str(&separator.join(" | "));
                        content.push('\n');
                        first_row = false;
                    }
                }
            }
            Err(e) => {
                content.push_str(&format!("（工作表解析失败: {}）\n", e));
            }
        }
        content.push('\n');
    }
    
    let file_type = if path.extension().and_then(|e| e.to_str())
        .map(|e| e.to_lowercase()) == Some("xls".to_string()) {
        FileType::Xls
    } else {
        FileType::Xlsx
    };

    Ok(FileContent {
        filename,
        file_type,
        content,
        metadata: Some(FileMetadata {
            size: file_size,
            pages: None,
            sheets: Some(sheet_names),
        }),
    })
}

/// 解析 PDF 文件
fn parse_pdf(path: &Path, filename: String, file_size: u64) -> Result<FileContent, FileError> {
    // 使用 lopdf 或直接读取文本流
    let bytes = std::fs::read(path).map_err(|e| FileError::IoError(e.to_string()))?;
    
    // 简单的PDF文本提取：查找文本流中的字符串
    let content = extract_pdf_text(&bytes);
    
    if content.trim().is_empty() {
        return Err(FileError::ParseError(
            "PDF文本提取失败，可能是扫描版PDF或加密PDF，暂不支持".to_string()
        ));
    }

    Ok(FileContent {
        filename,
        file_type: FileType::Pdf,
        content,
        metadata: Some(FileMetadata {
            size: file_size,
            pages: None,
            sheets: None,
        }),
    })
}

/// 从PDF字节流中提取文本（基础实现）
fn extract_pdf_text(bytes: &[u8]) -> String {
    let mut text = String::new();
    
    // 查找 BT...ET 文本块中的文本
    let content = String::from_utf8_lossy(bytes);
    let mut in_text_block = false;
    
    for line in content.lines() {
        let line = line.trim();
        if line == "BT" {
            in_text_block = true;
        } else if line == "ET" {
            in_text_block = false;
            text.push('\n');
        } else if in_text_block {
            // 提取 Tj 和 TJ 操作符中的文本
            if let Some(tj_text) = extract_tj_text(line) {
                text.push_str(&tj_text);
                text.push(' ');
            }
        }
    }
    
    // 清理文本
    let cleaned: Vec<&str> = text.lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    cleaned.join("\n")
}

/// 提取 Tj/TJ 操作符中的文本
fn extract_tj_text(line: &str) -> Option<String> {
    // 处理 (text) Tj 格式
    if line.ends_with(" Tj") || line.ends_with(" Tj ") {
        let content = line.trim_end_matches(" Tj").trim_end_matches("Tj").trim();
        if content.starts_with('(') && content.ends_with(')') {
            let inner = &content[1..content.len() - 1];
            return Some(decode_pdf_string(inner));
        }
    }
    
    // 处理 [(text)] TJ 格式
    if line.ends_with(" TJ") || line.ends_with(" TJ ") {
        let content = line.trim_end_matches(" TJ").trim_end_matches("TJ").trim();
        if content.starts_with('[') && content.ends_with(']') {
            let inner = &content[1..content.len() - 1];
            let mut result = String::new();
            // 提取括号内的文本
            let mut depth = 0i32;
            let mut start = 0usize;
            for (i, ch) in inner.char_indices() {
                match ch {
                    '(' => {
                        if depth == 0 { start = i + 1; }
                        depth += 1;
                    }
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            result.push_str(&decode_pdf_string(&inner[start..i]));
                        }
                    }
                    _ => {}
                }
            }
            if !result.is_empty() {
                return Some(result);
            }
        }
    }
    
    None
}

/// 解码PDF字符串（处理转义序列）
fn decode_pdf_string(s: &str) -> String {
    let mut result = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'n' => { result.push('\n'); i += 2; }
                b'r' => { result.push('\r'); i += 2; }
                b't' => { result.push('\t'); i += 2; }
                b'(' => { result.push('('); i += 2; }
                b')' => { result.push(')'); i += 2; }
                b'\\' => { result.push('\\'); i += 2; }
                _ => { result.push(bytes[i + 1] as char); i += 2; }
            }
        } else if bytes[i] >= 0x20 {
            result.push(bytes[i] as char);
            i += 1;
        } else {
            i += 1;
        }
    }
    result
}

/// 解析文本文件（TXT/LOG）
fn parse_text(path: &Path, filename: String, file_size: u64, file_type: FileType) -> Result<FileContent, FileError> {
    let bytes = std::fs::read(path).map_err(|e| FileError::IoError(e.to_string()))?;
    
    // 自动检测编码（UTF-8 或 GBK）
    let content = decode_text(&bytes);

    Ok(FileContent {
        filename,
        file_type,
        content,
        metadata: Some(FileMetadata {
            size: file_size,
            pages: None,
            sheets: None,
        }),
    })
}

/// 自动检测文本编码并解码
pub fn decode_text(bytes: &[u8]) -> String {
    // 先尝试 UTF-8
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    
    // 尝试 GBK/GB2312 解码
    let (decoded, _, had_errors) = encoding_rs::GBK.decode(bytes);
    if !had_errors {
        return decoded.to_string();
    }
    
    // 最后降级：用 UTF-8 lossy
    String::from_utf8_lossy(bytes).to_string()
}
