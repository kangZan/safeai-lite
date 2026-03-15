use crate::services::file_service::FileType;

#[allow(dead_code)]
/// 将普通文本转换为 Markdown 格式
pub fn to_markdown(content: &str, source_type: &FileType) -> String {
    match source_type {
        FileType::Xlsx | FileType::Xls => {
            // Excel 内容已在解析时格式化为 MD 表格，直接返回
            content.to_string()
        }
        FileType::Pdf => {
            // PDF 文本按段落格式化
            format_paragraphs(content)
        }
        _ => {
            // 其他文本类型按段落格式化
            format_paragraphs(content)
        }
    }
}

#[allow(dead_code)]
/// 将普通文本格式化为 Markdown 段落
pub fn format_paragraphs(content: &str) -> String {
    let mut result = String::new();
    let mut blank_count = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    result.trim().to_string()
}

#[allow(dead_code)]
/// 将普通文本格式化为 Markdown 脱敏结果展示文本（保留占位符）
pub fn format_desensitized_result(content: &str) -> String {
    format_paragraphs(content)
}
