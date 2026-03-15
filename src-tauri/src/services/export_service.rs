use serde::{Deserialize, Serialize};
use std::path::Path;

/// 导出格式枚举
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Docx,
    Xlsx,
    Pdf,
    Txt,
    Md,
}

/// 导出输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportInput {
    pub content: String,
    pub format: String, // "docx" | "xlsx" | "pdf" | "txt" | "md"
    pub path: String,
}

/// 导出错误类型
#[derive(Debug)]
pub enum ExportError {
    IoError(String),
    FormatError(String),
    UnsupportedFormat(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::IoError(msg) => write!(f, "文件写入失败: {}", msg),
            ExportError::FormatError(msg) => write!(f, "格式转换失败: {}", msg),
            ExportError::UnsupportedFormat(fmt) => write!(f, "不支持的导出格式: {}", fmt),
        }
    }
}

/// 导出文件 - 统一入口
pub fn export_file(input: ExportInput) -> Result<String, ExportError> {
    let path = Path::new(&input.path);
    
    match input.format.as_str() {
        "docx" => export_docx(&input.content, path),
        "xlsx" => export_xlsx(&input.content, path),
        "pdf" => export_pdf(&input.content, path),
        "txt" => export_txt(&input.content, path),
        "md" => export_md(&input.content, path),
        other => Err(ExportError::UnsupportedFormat(other.to_string())),
    }?;
    
    Ok(input.path.clone())
}

/// 导出为 Word (.docx)
fn export_docx(content: &str, path: &Path) -> Result<(), ExportError> {
    use docx_rs::{Docx, Paragraph, Run};
    
    let mut docx = Docx::new();
    
    for paragraph in content.split('\n') {
        // 跳过 Markdown 标题符号，转为纯文本段落
        let text = paragraph
            .trim_start_matches('#')
            .trim_start_matches('*')
            .trim_start_matches('-')
            .trim();
        
        if !text.is_empty() {
            docx = docx.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(text))
            );
        } else {
            // 添加空段落保持间距
            docx = docx.add_paragraph(Paragraph::new());
        }
    }
    
    let file = std::fs::File::create(path)
        .map_err(|e| ExportError::IoError(e.to_string()))?;
    
    docx.build().pack(file)
        .map_err(|e| ExportError::FormatError(format!("Word文档生成失败: {:?}", e)))?;
    
    Ok(())
}

/// 导出为 Excel (.xlsx)
fn export_xlsx(content: &str, path: &Path) -> Result<(), ExportError> {
    use rust_xlsxwriter::Workbook;
    
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    
    let mut current_sheet_row = 0u32;
    let mut sheet_count = 0u32;
    
    for line in content.lines() {
        let line = line.trim();
        
        if line.is_empty() {
            continue;
        }
        
        // 检测是否是 Markdown 二级标题（## Sheet名称）
        if line.starts_with("## ") {
            if sheet_count > 0 {
                // 已有sheet，新建一个 (简化处理：都写在同一个sheet，用空行分隔)
                current_sheet_row += 1;
                let _ = worksheet.write_string(current_sheet_row, 0, line.trim_start_matches('#').trim());
                current_sheet_row += 1;
            } else {
                let sheet_name = line.trim_start_matches('#').trim();
                let _ = worksheet.set_name(sheet_name);
            }
            sheet_count += 1;
            continue;
        }
        
        // 跳过表格分隔行（--- | --- 格式）
        if line.contains("---") && line.contains('|') {
            continue;
        }
        
        // 解析表格行（| col1 | col2 | 格式）
        if line.contains('|') {
            let cells: Vec<&str> = line.split('|')
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .collect();
            
            for (col, cell_value) in cells.iter().enumerate() {
                let _ = worksheet.write_string(current_sheet_row, col as u16, *cell_value);
            }
            current_sheet_row += 1;
        } else {
            // 非表格内容，直接写入第一列
            let _ = worksheet.write_string(current_sheet_row, 0, line);
            current_sheet_row += 1;
        }
    }
    
    workbook.save(path)
        .map_err(|e| ExportError::FormatError(format!("Excel文件生成失败: {}", e)))?;
    
    Ok(())
}

/// 导出为 PDF
fn export_pdf(content: &str, path: &Path) -> Result<(), ExportError> {
    use printpdf::*;
    use std::io::BufWriter;

    let (doc, page1, layer1) = PdfDocument::new("SafeAI-Lite 导出", Mm(210.0), Mm(297.0), "Layer 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);

    // 加载中文字体
    let font = load_chinese_font(&doc)
        .map_err(|e| ExportError::FormatError(format!("字体加载失败: {}", e)))?;

    let base_font_size = 12.0f32;
    let margin_left = Mm(20.0);
    let margin_right_mm = 20.0f32;
    let usable_width_mm = 210.0f32 - 20.0f32 - margin_right_mm; // 170mm
    let margin_top = 277.0f32;
    let margin_bottom = 20.0f32;

    let mut current_y = margin_top;
    let mut _current_page = page1;
    let mut current_layer_ref = current_layer;
    let mut page_count = 1u32;

    let new_page = |doc: &PdfDocumentReference,
                        current_page: &mut printpdf::PdfPageIndex,
                        layer_ref: &mut printpdf::PdfLayerReference,
                        y: &mut f32,
                        count: &mut u32| {
        let (np, nl) = doc.add_page(Mm(210.0), Mm(297.0), format!("Layer {}", *count + 1));
        *current_page = np;
        *layer_ref = doc.get_page(*current_page).get_layer(nl);
        *y = margin_top;
        *count += 1;
    };

    for line in content.lines() {
        // 处理 Markdown 标题格式
        let (display_text, font_sz) = if line.starts_with("# ") {
            (line.trim_start_matches("# ").to_string(), 18.0f32)
        } else if line.starts_with("## ") {
            (line.trim_start_matches("## ").to_string(), 15.0f32)
        } else if line.starts_with("### ") {
            (line.trim_start_matches("### ").to_string(), 13.0f32)
        } else {
            (line.to_string(), base_font_size)
        };

        // 按字符宽度估算每行最多字符数，自动折行
        // CJK 字符算 1 单位宽，ASCII 算 0.5 单位；字符宽约 font_sz * 0.353 mm
        let char_unit_width_mm = font_sz * 0.353;
        let max_units = (usable_width_mm / char_unit_width_mm) as usize;
        let wrapped_lines = wrap_text_pdf(&display_text, max_units);
        let line_height = font_sz * 0.353 * 1.6; // 行高 = 字号 * 1.6 倍，单位 mm

        for wrapped in &wrapped_lines {
            // 检查是否需要换页
            if current_y < margin_bottom {
                new_page(&doc, &mut _current_page, &mut current_layer_ref, &mut current_y, &mut page_count);
            }

            if !wrapped.trim().is_empty() {
                current_layer_ref.use_text(
                    wrapped.clone(),
                    font_sz,
                    margin_left,
                    Mm(current_y),
                    &font,
                );
            }
            current_y -= line_height;
        }
    }

    let file = std::fs::File::create(path)
        .map_err(|e| ExportError::IoError(e.to_string()))?;

    doc.save(&mut BufWriter::new(file))
        .map_err(|e| ExportError::FormatError(format!("PDF保存失败: {}", e)))?;

    Ok(())
}

/// 将文本按字符宽度单位折行（CJK=1单位，ASCII=0.5单位）
fn wrap_text_pdf(text: &str, max_units: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut units = 0.0f32;

    for ch in text.chars() {
        let w = if is_cjk_char(ch) { 1.0f32 } else { 0.5f32 };
        if units + w > max_units as f32 && !current.is_empty() {
            lines.push(current.clone());
            current.clear();
            units = 0.0;
        }
        current.push(ch);
        units += w;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// 判断是否为 CJK 全角字符
fn is_cjk_char(ch: char) -> bool {
    let c = ch as u32;
    matches!(c,
        0x4E00..=0x9FFF   // CJK 统一汉字
        | 0x3400..=0x4DBF // CJK 扩展 A
        | 0xF900..=0xFAFF // CJK 兼容汉字
        | 0x3000..=0x303F // CJK 标点
        | 0xFF00..=0xFFEF // 全角字母/标点
        | 0x2E80..=0x2EFF // CJK 部首
    )
}

/// 加载中文字体
fn load_chinese_font(doc: &printpdf::PdfDocumentReference) -> Result<printpdf::IndirectFontRef, String> {
    // 尝试加载系统中文字体
    let font_paths = get_chinese_font_paths();
    
    for font_path in &font_paths {
        if Path::new(font_path).exists() {
            let font_bytes = std::fs::read(font_path)
                .map_err(|e| format!("读取字体失败: {}", e))?;
            return doc.add_external_font(std::io::Cursor::new(font_bytes))
                .map_err(|e| format!("加载字体失败: {}", e));
        }
    }
    
    Err("未找到可用的中文字体，请确保系统已安装中文字体".to_string())
}

/// 获取各平台中文字体路径列表
fn get_chinese_font_paths() -> Vec<String> {
    let mut paths = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        paths.push(r"C:\Windows\Fonts\simhei.ttf".to_string());
        paths.push(r"C:\Windows\Fonts\simsun.ttc".to_string());
        paths.push(r"C:\Windows\Fonts\msyh.ttc".to_string());
        paths.push(r"C:\Windows\Fonts\msyhbd.ttc".to_string());
        paths.push(r"C:\Windows\Fonts\arial.ttf".to_string()); // 降级选项
    }
    
    #[cfg(target_os = "macos")]
    {
        paths.push("/System/Library/Fonts/STHeiti Medium.ttc".to_string());
        paths.push("/System/Library/Fonts/PingFang.ttc".to_string());
        paths.push("/Library/Fonts/Arial Unicode.ttf".to_string());
    }
    
    #[cfg(target_os = "linux")]
    {
        paths.push("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc".to_string());
        paths.push("/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc".to_string());
        paths.push("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".to_string());
    }
    
    paths
}

/// 导出为 TXT
fn export_txt(content: &str, path: &Path) -> Result<(), ExportError> {
    std::fs::write(path, content.as_bytes())
        .map_err(|e| ExportError::IoError(e.to_string()))?;
    Ok(())
}

/// 导出为 Markdown (.md)
fn export_md(content: &str, path: &Path) -> Result<(), ExportError> {
    std::fs::write(path, content.as_bytes())
        .map_err(|e| ExportError::IoError(e.to_string()))?;
    Ok(())
}
