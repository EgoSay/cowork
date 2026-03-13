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

use features::skills::commands as skills_commands;
use features::usage::commands as usage_commands;
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
            skills_commands::scan_all_tools,
            skills_commands::scan_tool,
            skills_commands::get_skill_detail,
            skills_commands::push_skill,
            skills_commands::disable_skill,
            skills_commands::enable_skill,
            skills_commands::delete_skill,
            skills_commands::reveal_in_finder,
            skills_commands::get_tool_configs,
            skills_commands::update_tool_config,
            usage_commands::get_usage_data,
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
