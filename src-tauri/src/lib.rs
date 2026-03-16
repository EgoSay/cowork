/**
 * [INPUT]: 依赖 tauri, tauri_plugin_opener, tauri_plugin_shell
 * [OUTPUT]: 对外提供 run() 函数, ProviderLock, ProjectsLock
 * [POS]: crate 入口，声明所有模块，组装 Tauri Builder，定义全局并发锁
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
mod config;
mod features;
mod shared;
mod types;

use features::skills::commands;
use features::usage::commands as usage_commands;
use features::providers::commands as provider_commands;
use features::projects::commands as project_commands;
use std::sync::Mutex;

/// 供应商操作的并发锁，防止 load→save 竞态
pub struct ProviderLock(pub Mutex<()>);

/// 项目操作的并发锁，防止 scan/annotate 竞态
pub struct ProjectsLock(pub Mutex<()>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(ProviderLock(Mutex::new(())))
        .manage(ProjectsLock(Mutex::new(())))
        .invoke_handler(tauri::generate_handler![
            commands::scan_all_tools,
            commands::scan_tool,
            commands::get_skill_detail,
            commands::push_skill,
            commands::disable_skill,
            commands::enable_skill,
            commands::delete_skill,
            commands::reveal_in_finder,
            commands::save_skill_content,
            commands::get_tool_configs,
            commands::update_tool_config,
            usage_commands::get_usage_data,
            provider_commands::get_providers,
            provider_commands::switch_provider,
            provider_commands::add_provider,
            provider_commands::update_provider,
            provider_commands::remove_provider,
            provider_commands::read_claude_env,
            project_commands::scan_projects,
            project_commands::annotate_session,
            project_commands::get_annotations,
            project_commands::remove_annotation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CoWork");
}
