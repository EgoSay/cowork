/**
 * [INPUT]: 依赖 types::PushResult, config::AppConfig, types::Tool
 * [OUTPUT]: 对外提供 push_to_tool() 符号链接推送，skill_dir_name() 获取技能目录名
 * [POS]: skills 的推送引擎，通过 symlink 实现一处维护多工具同步
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::PushResult;
use crate::config::AppConfig;
use crate::types::Tool;
use std::path::Path;

/// 从 SKILL.md 路径推导出技能目录的真实路径和名称
/// 例: ~/.claude/skills/brand-namer/SKILL.md → (~/.skillshub/brand-namer, "brand-namer")
pub fn skill_dir_name(file_path: &Path) -> Option<(std::path::PathBuf, String)> {
    let skill_dir = file_path.parent()?;
    let real_dir = skill_dir.canonicalize().ok()?;
    let name = real_dir.file_name()?.to_str()?.to_string();
    Some((real_dir, name))
}

/// 在目标工具的 skills 目录创建符号链接，指向技能的真实目录
pub fn push_to_tool(
    source_path: &Path,
    target_tool: &Tool,
    config: &AppConfig,
) -> PushResult {
    // 解析技能目录的真实路径和名称
    let (real_dir, skill_name) = match skill_dir_name(source_path) {
        Some(v) => v,
        None => return PushResult::Error {
            message: "Cannot resolve skill directory".into(),
        },
    };

    // 获取目标工具的 skills 目录
    let target_dir = match config.get_skills_dir(target_tool) {
        Some(dir) => dir,
        None => return PushResult::Error {
            message: format!("No skills directory configured for {}", target_tool),
        },
    };

    // 确保目标目录存在
    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        return PushResult::Error { message: e.to_string() };
    }

    // 创建 symlink: target_dir/skill_name → real_dir
    let link_path = target_dir.join(&skill_name);

    // 检查是否已存在（包括断链的 symlink）
    if link_path.symlink_metadata().is_ok() {
        return PushResult::AlreadyExists {
            path: link_path.to_string_lossy().to_string(),
        };
    }

    match std::os::unix::fs::symlink(&real_dir, &link_path) {
        Ok(_) => PushResult::Success {
            path: link_path.to_string_lossy().to_string(),
        },
        Err(e) => PushResult::Error { message: e.to_string() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn test_config(target_dir: &Path) -> AppConfig {
        let mut tools = HashMap::new();
        tools.insert("cursor".into(), crate::config::ToolConfig {
            skills_dir: target_dir.to_string_lossy().to_string(),
            scan_patterns: vec!["*.mdc".into()],
        });
        AppConfig { tools }
    }

    #[test]
    fn push_creates_symlink_to_skill_dir() {
        let tmp = TempDir::new().unwrap();

        // 模拟 skillshub 中的真实目录
        let hub_dir = tmp.path().join("skillshub/my-skill");
        fs::create_dir_all(&hub_dir).unwrap();
        fs::write(hub_dir.join("SKILL.md"), "---\nname: my-skill\n---\n").unwrap();

        let target_dir = tmp.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();

        let config = test_config(&target_dir);
        let result = push_to_tool(&hub_dir.join("SKILL.md"), &Tool::Cursor, &config);

        match result {
            PushResult::Success { path } => {
                let link = std::path::Path::new(&path);
                assert!(link.is_symlink());
                // canonicalize 消除 macOS /var → /private/var 差异
                let target = fs::read_link(link).unwrap().canonicalize().unwrap();
                assert_eq!(target, hub_dir.canonicalize().unwrap());
            }
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn push_detects_existing_link() {
        let tmp = TempDir::new().unwrap();

        let hub_dir = tmp.path().join("skillshub/dup-skill");
        fs::create_dir_all(&hub_dir).unwrap();
        fs::write(hub_dir.join("SKILL.md"), "content").unwrap();

        let target_dir = tmp.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();

        // 预先创建同名链接
        std::os::unix::fs::symlink(&hub_dir, target_dir.join("dup-skill")).unwrap();

        let config = test_config(&target_dir);
        let result = push_to_tool(&hub_dir.join("SKILL.md"), &Tool::Cursor, &config);

        match result {
            PushResult::AlreadyExists { .. } => {}
            other => panic!("Expected AlreadyExists, got {:?}", other),
        }
    }

    #[test]
    fn push_to_unconfigured_tool_returns_error() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("hub/skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "content").unwrap();

        let config = AppConfig { tools: HashMap::new() };
        let result = push_to_tool(&source.join("SKILL.md"), &Tool::Trae, &config);

        match result {
            PushResult::Error { message } => {
                assert!(message.contains("No skills directory"));
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn skill_dir_name_resolves_symlink() {
        let tmp = TempDir::new().unwrap();

        // 真实目录
        let real = tmp.path().join("hub/brand-namer");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("SKILL.md"), "skill").unwrap();

        // 创建符号链接
        let link = tmp.path().join("link-brand-namer");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let (resolved, name) = skill_dir_name(&link.join("SKILL.md")).unwrap();
        assert_eq!(name, "brand-namer");
        assert_eq!(resolved, real.canonicalize().unwrap());
    }
}
