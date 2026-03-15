use crate::db::get_connection;
use crate::models::session::{RestoreInput, RestoreResult};
use regex::Regex;

pub fn restore(input: RestoreInput) -> Result<RestoreResult, String> {
    // 获取会话的映射关系
    let mappings = get_mappings(&input.session_id)?;
    
    if mappings.is_empty() {
        return Ok(RestoreResult {
            content: input.content,
        });
    }
    
    // 执行还原
    let restored_content = perform_restore(&input.content, &mappings)?;
    
    Ok(RestoreResult {
        content: restored_content,
    })
}

fn get_mappings(session_id: &str) -> Result<Vec<(String, String)>, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT placeholder, original_value FROM desensitize_mappings WHERE session_id = ?1"
    ).map_err(|e| e.to_string())?;
    
    let mappings = stmt.query_map([session_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| e.to_string())?;
    
    mappings.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn perform_restore(content: &str, mappings: &[(String, String)]) -> Result<String, String> {
    let mut result = content.to_string();
    
    // 按占位符长度降序排序，避免短占位符干扰长占位符
    let mut sorted_mappings: Vec<_> = mappings.iter().collect();
    sorted_mappings.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    
    for (placeholder, original) in sorted_mappings {
        // 转义正则特殊字符
        let escaped_placeholder = regex::escape(placeholder);
        let pattern = format!("{}", escaped_placeholder);
        
        if let Ok(re) = Regex::new(&pattern) {
            result = re.replace_all(&result, original.as_str()).to_string();
        }
    }
    
    Ok(result)
}
