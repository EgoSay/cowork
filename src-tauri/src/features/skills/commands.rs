/**
 * [INPUT]: 依赖 scanner, hub, types, config
 * [OUTPUT]: 对外提供所有 #[tauri::command] 函数
 * [POS]: skills 功能的 Tauri IPC 接口层
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::scanner;
use super::hub;
use super::types::{EnableResult, MigrateReport, PushTarget, SkillDetail, SkillMeta, SyncReport, VerifyReport};
use crate::config::AppConfig;
use crate::types::Tool;
use std::path::Path;

#[tauri::command]
pub async fn scan_all_tools() -> Result<Vec<SkillMeta>, String> {
    let config = AppConfig::load();
    Ok(scanner::scan_all(&config))
}

#[tauri::command]
pub async fn scan_tool(tool: Tool) -> Result<Vec<SkillMeta>, String> {
    let config = AppConfig::load();
    Ok(scanner::scan_one(&config, &tool))
}

#[tauri::command]
pub async fn get_skill_detail(meta: SkillMeta) -> Result<SkillDetail, String> {
    let content = std::fs::read_to_string(&meta.file_path)
        .map_err(|e| format!("Failed to read {}: {}", meta.file_path, e))?;

    let config = AppConfig::load();
    let dir_name = hub::skill_dir_name(Path::new(&meta.file_path))
        .map(|(_, name)| name);
    let push_status = hub::ALL_TOOLS
        .iter()
        .map(|tool| {
            let dir = config.get_skills_dir(tool);
            let deployed = match (&dir_name, &dir) {
                (Some(name), Some(d)) => d.join(name).symlink_metadata().is_ok(),
                _ => false,
            };
            PushTarget {
                tool: *tool,
                deployed,
                target_path: dir.map(|p| p.to_string_lossy().to_string()),
            }
        })
        .collect();

    Ok(SkillDetail { meta, content, push_status, dir_name })
}

// ── Hub 操作 ──

#[tauri::command]
pub async fn enable_skill(skill_name: String, targets: Vec<Tool>) -> Result<Vec<EnableResult>, String> {
    let config = AppConfig::load();
    let mut results = Vec::new();
    for tool in &targets {
        results.push(hub::enable(&skill_name, tool, &config)?);
    }
    Ok(results)
}

#[tauri::command]
pub async fn disable_skill(skill_name: String, targets: Vec<Tool>) -> Result<(), String> {
    let config = AppConfig::load();
    for tool in &targets {
        hub::disable(&skill_name, tool, &config)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn install_skill(source_path: String) -> Result<String, String> {
    let config = AppConfig::load();
    let expanded = crate::shared::fs_utils::expand_tilde(&source_path);
    hub::install(&expanded, &config)
}

#[tauri::command]
pub async fn delete_skill(skill_name: String) -> Result<(), String> {
    let config = AppConfig::load();
    hub::delete(&skill_name, &config)
}

#[tauri::command]
pub async fn migrate_hub(new_path: String) -> Result<MigrateReport, String> {
    let config = AppConfig::load();
    let old_path = config.get_skillshub_dir();
    let new_expanded = crate::shared::fs_utils::expand_tilde(&new_path);
    let report = hub::migrate(&old_path, &new_expanded, &config)?;

    let mut config = config;
    if let Some(hub_config) = config.tools.get_mut("skillshub") {
        hub_config.skills_dir = new_path;
    }
    config.save()?;

    Ok(report)
}

#[tauri::command]
pub async fn sync_skills() -> Result<SyncReport, String> {
    let config = AppConfig::load();
    hub::sync(&config)
}

#[tauri::command]
pub async fn verify_skills() -> Result<VerifyReport, String> {
    let config = AppConfig::load();
    hub::verify(&config)
}

// ── 保留不变 ──

#[tauri::command]
pub async fn reveal_in_finder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 路径边界校验：防止任意文件写入
fn is_within_skills_dirs(path: &Path, config: &AppConfig) -> bool {
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let hub_dir = Some(config.get_skillshub_dir());
    hub::ALL_TOOLS
        .iter()
        .filter_map(|t| config.get_skills_dir(t))
        .chain(hub_dir)
        .any(|dir| {
            dir.canonicalize()
                .map(|d| canonical.starts_with(&d))
                .unwrap_or(false)
        })
}

#[tauri::command]
pub async fn save_skill_content(file_path: String, content: String) -> Result<(), String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    let config = AppConfig::load();
    if !is_within_skills_dirs(path, &config) {
        return Err(format!("Path is outside skills directories: {}", file_path));
    }
    std::fs::write(path, &content)
        .map_err(|e| format!("Failed to write {}: {}", file_path, e))
}

#[tauri::command]
pub async fn get_tool_configs() -> Result<AppConfig, String> {
    Ok(AppConfig::load())
}

#[tauri::command]
pub async fn update_tool_config(tool_key: String, skills_dir: String) -> Result<(), String> {
    let mut config = AppConfig::load();
    if let Some(tool_config) = config.tools.get_mut(&tool_key) {
        tool_config.skills_dir = skills_dir;
    }
    config.save()
}
