/**
 * [INPUT]: 依赖 scanner, pusher, types, config
 * [OUTPUT]: 对外提供所有 #[tauri::command] 函数（含 save_skill_content）
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
    let skill_name = super::pusher::skill_dir_name(Path::new(&meta.file_path))
        .map(|(_, name)| name);
    let push_status = [Tool::ClaudeCode, Tool::Codex, Tool::Cursor, Tool::Trae]
        .iter()
        .map(|tool| {
            let dir = config.get_skills_dir(tool);
            let deployed = skill_name.as_ref().map_or(false, |name| {
                dir.as_ref().map_or(false, |d| d.join(name).symlink_metadata().is_ok())
            });
            PushTarget {
                tool: *tool,
                deployed,
                target_path: dir.map(|p| p.to_string_lossy().to_string()),
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
    let config = AppConfig::load();
    if !is_within_skills_dirs(path, &config) {
        return Err(format!("Path is outside skills directories: {}", file_path));
    }
    // 符号链接优先：remove_file 只删链接本身，不删目标
    if path.is_symlink() {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    } else if path.is_dir() {
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

// ---- 路径边界校验：防止任意文件写入 ----
fn is_within_skills_dirs(path: &Path, config: &AppConfig) -> bool {
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    // 四个工具目录 + skillshub 均为合法写入区域
    let hub_dir = config.tools.get("skillshub")
        .map(|c| crate::shared::fs_utils::expand_tilde(&c.skills_dir));
    [Tool::ClaudeCode, Tool::Codex, Tool::Cursor, Tool::Trae]
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
