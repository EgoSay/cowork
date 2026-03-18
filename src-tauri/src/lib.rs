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
use features::providers::commands as provider_commands;
use std::sync::Mutex;

/// 供应商操作的并发锁，防止 load→save 竞态
pub struct ProviderLock(pub Mutex<()>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(ProviderLock(Mutex::new(())))
        .invoke_handler(tauri::generate_handler![
            commands::scan_all_tools,
            commands::scan_tool,
            commands::get_skill_detail,
            commands::enable_skill,
            commands::disable_skill,
            commands::delete_skill,
            commands::migrate_hub,
            commands::sync_skills,
            commands::verify_skills,
            commands::reveal_in_finder,
            commands::save_skill_content,
            commands::get_tool_configs,
            commands::update_tool_config,
            provider_commands::get_providers,
            provider_commands::switch_provider,
            provider_commands::add_provider,
            provider_commands::update_provider,
            provider_commands::remove_provider,
            provider_commands::read_claude_env,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CoWork");
}
