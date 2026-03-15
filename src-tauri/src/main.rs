#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod models;
mod services;
mod utils;

use db::init_database;
use services::ner_service::init_ner;
use tauri::Manager;

fn main() {
    // 初始化数据库
    if let Err(e) = init_database() {
        eprintln!("数据库初始化失败: {}", e);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            // 实体相关命令
            commands::entity::entity_get_all,
            commands::entity::entity_get_builtin,
            commands::entity::entity_get_custom,
            commands::entity::entity_toggle,
            commands::entity::entity_update_strategy,
            commands::entity::entity_create,
            commands::entity::entity_update,
            commands::entity::entity_update_synonyms,
            commands::entity::entity_delete,
            // 脱敏相关命令
            commands::desensitize::desensitize_scan,
            commands::desensitize::desensitize_execute,
            commands::desensitize::ner_get_status,
            // 还原相关命令
            commands::restore::restore_execute,
            // 文件处理命令
            commands::file::file_read,
            commands::file::file_export,
            // 会话管理命令
            commands::session::session_get_all,
            commands::session::session_get_by_id,
            commands::session::session_delete,
            commands::session::session_clear_all,
        ])
        .setup(|app| {
            // 设置窗口图标（开发模式和发布模式均生效）
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_icon(tauri::include_image!("icons/icon.png"));
            }

            // 初始化 NER 模型（异步，不阻塞启动）
            let resource_dir = app.path().resource_dir()
                .expect("failed to get resource dir");
            let model_path = resource_dir.join("ner").join("model_quantized.onnx");
            let tokenizer_path = resource_dir.join("ner").join("tokenizer.json");
            if model_path.exists() && tokenizer_path.exists() {
                std::thread::spawn(move || {
                    match init_ner(&model_path, &tokenizer_path) {
                        Ok(_) => println!("[NER] 模型加载成功"),
                        Err(e) => eprintln!("[NER] 模型加载失败: {}", e),
                    }
                });
            } else {
                eprintln!("[NER] 模型文件不存在，跳过 NER 初始化: {:?}", model_path);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
