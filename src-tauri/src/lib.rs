/**
 * [INPUT]: 依赖 tauri, tauri_plugin_opener, tauri_plugin_shell
 * [OUTPUT]: 对外提供 run() 函数启动 Tauri 应用
 * [POS]: crate 入口，声明所有模块，组装 Tauri Builder
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
mod config;
mod features;
mod shared;
mod types;

use features::skills::commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::scan_all_tools,
            commands::scan_tool,
            commands::get_skill_detail,
            commands::push_skill,
            commands::disable_skill,
            commands::enable_skill,
            commands::delete_skill,
            commands::reveal_in_finder,
            commands::get_tool_configs,
            commands::update_tool_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CoWork");
}
