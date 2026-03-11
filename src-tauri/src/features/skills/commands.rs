/**
 * [INPUT]: 依赖 scanner, pusher, types, config
 * [OUTPUT]: 对外提供所有 #[tauri::command] 函数
 * [POS]: skills 功能的 Tauri IPC 接口层
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::scanner;
use super::types::{PushResult, PushTarget, SkillDetail, SkillMeta};
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
    let push_status = [Tool::ClaudeCode, Tool::Codex, Tool::Cursor, Tool::Trae]
        .iter()
        .map(|tool| {
            let deployed = if let Some(dir) = config.get_skills_dir(tool) {
                let file_name = Path::new(&meta.file_path).file_name()
                    .unwrap_or_default();
                dir.join(file_name).exists()
            } else {
                false
            };
            PushTarget {
                tool: tool.clone(),
                deployed,
                target_path: config.get_skills_dir(tool)
                    .map(|p| p.to_string_lossy().to_string()),
            }
        })
        .collect();

    Ok(SkillDetail { meta, content, push_status })
}

#[tauri::command]
pub async fn push_skill(
    file_path: String,
    targets: Vec<Tool>,
) -> Result<Vec<PushResult>, String> {
    let config = AppConfig::load();
    let source = Path::new(&file_path);
    let results: Vec<PushResult> = targets.iter()
        .map(|tool| super::pusher::push_to_tool(source, tool, &config))
        .collect();
    Ok(results)
}

#[tauri::command]
pub async fn disable_skill(file_path: String) -> Result<(), String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err("File not found".into());
    }
    let disabled_path = path.with_extension("disabled");
    std::fs::rename(path, &disabled_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn enable_skill(file_path: String) -> Result<(), String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err("File not found".into());
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext == "disabled" {
            let enabled_path = path.with_file_name(
                path.file_stem().unwrap_or_default()
            );
            std::fs::rename(path, &enabled_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_skill(file_path: String) -> Result<(), String> {
    let path = Path::new(&file_path);
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    }
}

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

#[tauri::command]
pub async fn get_tool_configs() -> Result<AppConfig, String> {
    Ok(AppConfig::load())
}

#[tauri::command]
pub async fn update_tool_config(
    tool_key: String,
    skills_dir: String,
) -> Result<(), String> {
    let mut config = AppConfig::load();
    if let Some(tool_config) = config.tools.get_mut(&tool_key) {
        tool_config.skills_dir = skills_dir;
    }
    config.save()
}
