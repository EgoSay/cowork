/**
 * [INPUT]: 依赖 config::AppConfig, types::{Tool, EnableResult, VerifyReport, SyncReport, MigrateReport}
 * [OUTPUT]: 对外提供 enable, disable, delete, migrate, sync, verify, skill_dir_name
 * [POS]: skills 的中央管理器，通过 symlink 管理 skill 生命周期，取代 pusher
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::EnableResult;
use crate::config::AppConfig;
use crate::types::Tool;
use std::path::{Path, PathBuf};

// ── 工具函数 ──

/// 从 SKILL.md 路径推导出技能目录的真实路径和名称
pub fn skill_dir_name(file_path: &Path) -> Option<(PathBuf, String)> {
    let skill_dir = file_path.parent()?;
    let real_dir = skill_dir.canonicalize().ok()?;
    let name = real_dir.file_name()?.to_str()?.to_string();
    Some((real_dir, name))
}

/// 所有工具枚举（用于遍历）
const ALL_TOOLS: [Tool; 4] = [Tool::ClaudeCode, Tool::Codex, Tool::Cursor, Tool::Trae];

// ── enable ──

/// 在 tool 的 skills 目录创建指向 skillshub 的 symlink
pub fn enable(skill_dir_name: &str, tool: &Tool, config: &AppConfig) -> Result<EnableResult, String> {
    let hub_dir = config.get_skillshub_dir();
    let source = hub_dir.join(skill_dir_name);
    if !source.exists() {
        return Err(format!("Skill '{}' not found in skillshub", skill_dir_name));
    }

    let target_dir = config.get_skills_dir(tool)
        .ok_or_else(|| format!("No skills directory configured for {}", tool))?;
    std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let link_path = target_dir.join(skill_dir_name);

    if link_path.symlink_metadata().is_ok() {
        return Ok(EnableResult::AlreadyEnabled {
            path: link_path.to_string_lossy().to_string(),
        });
    }

    std::os::unix::fs::symlink(&source, &link_path)
        .map_err(|e| e.to_string())?;

    Ok(EnableResult::Success {
        path: link_path.to_string_lossy().to_string(),
    })
}

// ── disable ──

/// 删除 tool 目录里的 symlink（skill 仍在 skillshub）
pub fn disable(skill_dir_name: &str, tool: &Tool, config: &AppConfig) -> Result<(), String> {
    let target_dir = config.get_skills_dir(tool)
        .ok_or_else(|| format!("No skills directory configured for {}", tool))?;
    let link_path = target_dir.join(skill_dir_name);

    if link_path.symlink_metadata().is_ok() && link_path.is_symlink() {
        std::fs::remove_file(&link_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── delete ──

/// 从 skillshub 删除 skill 目录 + 清理所有工具里指向它的 symlink
pub fn delete(skill_dir_name: &str, config: &AppConfig) -> Result<(), String> {
    for tool in &ALL_TOOLS {
        let _ = disable(skill_dir_name, tool, config);
    }

    let hub_dir = config.get_skillshub_dir();
    let skill_path = hub_dir.join(skill_dir_name);
    if skill_path.exists() {
        std::fs::remove_dir_all(&skill_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn test_config(hub_dir: &Path, tool_dir: &Path) -> AppConfig {
        let mut tools = HashMap::new();
        tools.insert("skillshub".into(), crate::config::ToolConfig {
            skills_dir: hub_dir.to_string_lossy().to_string(),
            scan_patterns: vec!["*/SKILL.md".into()],
        });
        tools.insert("claude_code".into(), crate::config::ToolConfig {
            skills_dir: tool_dir.to_string_lossy().to_string(),
            scan_patterns: vec!["*/SKILL.md".into()],
        });
        AppConfig { tools }
    }

    #[test]
    fn enable_creates_symlink() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("my-skill")).unwrap();
        fs::write(hub.join("my-skill/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        let result = enable("my-skill", &Tool::ClaudeCode, &config).unwrap();
        assert!(matches!(result, EnableResult::Success { .. }));
        assert!(tool.join("my-skill").is_symlink());
    }

    #[test]
    fn enable_returns_already_enabled() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("my-skill")).unwrap();
        fs::write(hub.join("my-skill/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        enable("my-skill", &Tool::ClaudeCode, &config).unwrap();
        let result = enable("my-skill", &Tool::ClaudeCode, &config).unwrap();
        assert!(matches!(result, EnableResult::AlreadyEnabled { .. }));
    }

    #[test]
    fn enable_nonexistent_skill_returns_error() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(&hub).unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        let result = enable("ghost", &Tool::ClaudeCode, &config);
        assert!(result.is_err());
    }

    #[test]
    fn disable_removes_symlink() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("my-skill")).unwrap();
        fs::write(hub.join("my-skill/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        enable("my-skill", &Tool::ClaudeCode, &config).unwrap();
        assert!(tool.join("my-skill").is_symlink());

        disable("my-skill", &Tool::ClaudeCode, &config).unwrap();
        assert!(!tool.join("my-skill").exists());
    }

    #[test]
    fn disable_noop_if_not_exists() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(&hub).unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        disable("nonexistent", &Tool::ClaudeCode, &config).unwrap();
    }

    #[test]
    fn disable_does_not_delete_real_dir() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(&hub).unwrap();
        fs::create_dir_all(tool.join("real-skill")).unwrap();
        fs::write(tool.join("real-skill/SKILL.md"), "content").unwrap();

        let config = test_config(&hub, &tool);
        disable("real-skill", &Tool::ClaudeCode, &config).unwrap();
        assert!(tool.join("real-skill").exists());
    }

    #[test]
    fn delete_removes_hub_dir_and_symlinks() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("my-skill")).unwrap();
        fs::write(hub.join("my-skill/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        enable("my-skill", &Tool::ClaudeCode, &config).unwrap();

        delete("my-skill", &config).unwrap();
        assert!(!hub.join("my-skill").exists());
        assert!(!tool.join("my-skill").exists());
    }
}
