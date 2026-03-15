use crate::db::get_connection;
use crate::models::session::{MappingItem, SessionDetail, SessionListItem};

/// 获取所有会话列表（按时间倒序），含映射数量和预览
pub fn get_all_sessions() -> Result<Vec<SessionListItem>, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.name, s.status, s.desensitized_content, s.created_at,
                    COUNT(m.id) as mapping_count
             FROM sessions s
             LEFT JOIN desensitize_mappings m ON m.session_id = s.id
             GROUP BY s.id
             ORDER BY s.created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let items = stmt
        .query_map([], |row| {
            let desensitized: String = row.get(3)?;
            // 预览取前100字符
            let preview: String = desensitized.chars().take(100).collect();
            Ok(SessionListItem {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                desensitized_content: desensitized,
                created_at: row.get(4)?,
                mapping_count: row.get::<_, i64>(5)? as u32,
                preview,
            })
        })
        .map_err(|e| e.to_string())?;

    items
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// 获取会话详情（含映射关系）
pub fn get_session_by_id(id: &str) -> Result<SessionDetail, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;

    // 查询会话基本信息
    let session = conn
        .query_row(
            "SELECT id, name, original_content, desensitized_content, created_at, updated_at
             FROM sessions WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    // 查询映射数据
    let mut stmt = conn
        .prepare(
            "SELECT id, placeholder, original_value, entity_name
             FROM desensitize_mappings WHERE session_id = ?1
             ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let mappings = stmt
        .query_map([id], |row| {
            Ok(MappingItem {
                id: row.get(0)?,
                placeholder: row.get(1)?,
                original_value: row.get(2)?,
                entity_name: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mappings: Vec<MappingItem> = mappings
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(SessionDetail {
        id: session.0,
        name: session.1,
        original_content: session.2,
        desensitized_content: session.3,
        mappings,
        created_at: session.4,
        updated_at: session.5,
    })
}

/// 删除单个会话（级联删除映射）
pub fn delete_session(id: &str) -> Result<bool, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;

    // 开启外键支持（rusqlite 默认关闭）
    conn.execute("PRAGMA foreign_keys = ON", [])
        .map_err(|e| e.to_string())?;

    let affected = conn
        .execute("DELETE FROM sessions WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;

    Ok(affected > 0)
}

/// 清空所有会话（级联删除所有映射）
pub fn clear_all_sessions() -> Result<bool, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;

    conn.execute("PRAGMA foreign_keys = ON", [])
        .map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM sessions", [])
        .map_err(|e| e.to_string())?;

    Ok(true)
}
