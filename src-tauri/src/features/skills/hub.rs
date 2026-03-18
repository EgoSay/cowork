/**
 * [INPUT]: 依赖 config::AppConfig, types::{Tool, EnableResult, VerifyReport, SyncReport, MigrateReport}
 * [OUTPUT]: 对外提供 enable, disable, delete, migrate, sync, verify, skill_dir_name
 * [POS]: skills 的中央管理器，通过 symlink 管理 skill 生命周期，取代 pusher
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::{EnableResult, MigrateReport, SyncReport, VerifyReport};
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

// ── verify ──

/// 遍历所有工具目录的 symlink，检查指向是否存在且在 skillshub 内
pub fn verify(config: &AppConfig) -> Result<VerifyReport, String> {
    let hub_dir = config.get_skillshub_dir();
    let hub_canonical = hub_dir.canonicalize().unwrap_or_else(|_| hub_dir.clone());
    let mut report = VerifyReport {
        ok: Vec::new(),
        broken: Vec::new(),
    };

    for tool in &ALL_TOOLS {
        let tool_dir = match config.get_skills_dir(tool) {
            Some(d) => d,
            None => continue,
        };
        if !tool_dir.exists() {
            continue;
        }
        let entries = std::fs::read_dir(&tool_dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_symlink() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            match std::fs::read_link(&path) {
                Ok(target) => {
                    if !target.exists() {
                        report.broken.push((*tool, name, "Target does not exist".into()));
                    } else {
                        let target_canonical = target.canonicalize()
                            .unwrap_or_else(|_| target.clone());
                        if target_canonical.starts_with(&hub_canonical) {
                            report.ok.push((*tool, name));
                        } else {
                            report.broken.push((
                                *tool,
                                name,
                                format!("Target not in skillshub: {}", target.display()),
                            ));
                        }
                    }
                }
                Err(e) => {
                    report.broken.push((*tool, name, format!("Cannot read link: {}", e)));
                }
            }
        }
    }

    Ok(report)
}

// ── sync ──

/// 扫描各工具目录，找到非 symlink 且含 SKILL.md 的目录 → 复制到 skillshub → 替换为 symlink
/// 范围限制: 仅处理目录型 skill（含 SKILL.md），不处理 .mdc/.rules 等独立文件格式
/// 原子顺序: copy → rename .bak → symlink → verify → delete .bak / rollback
pub fn sync(config: &AppConfig) -> Result<SyncReport, String> {
    let hub_dir = config.get_skillshub_dir();
    std::fs::create_dir_all(&hub_dir).map_err(|e| e.to_string())?;

    let mut report = SyncReport {
        imported: Vec::new(),
        skipped: Vec::new(),
        errors: Vec::new(),
    };

    for tool in &ALL_TOOLS {
        let tool_dir = match config.get_skills_dir(tool) {
            Some(d) if d.exists() => d,
            _ => continue,
        };

        let entries = match std::fs::read_dir(&tool_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_symlink() || !path.is_dir() {
                continue;
            }
            if !path.join("SKILL.md").exists() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            let hub_target = hub_dir.join(&name);

            if hub_target.exists() {
                report.skipped.push((*tool, name, "Already exists in skillshub".into()));
                continue;
            }

            let bak_path = path.with_file_name(format!("{}.bak", name));

            if let Err(e) = copy_dir_recursive(&path, &hub_target) {
                report.errors.push(format!("{}/{}: copy failed: {}", tool, name, e));
                let _ = std::fs::remove_dir_all(&hub_target);
                continue;
            }

            if let Err(e) = std::fs::rename(&path, &bak_path) {
                report.errors.push(format!("{}/{}: rename to .bak failed: {}", tool, name, e));
                let _ = std::fs::remove_dir_all(&hub_target);
                continue;
            }

            if let Err(e) = std::os::unix::fs::symlink(&hub_target, &path) {
                let _ = std::fs::rename(&bak_path, &path);
                let _ = std::fs::remove_dir_all(&hub_target);
                report.errors.push(format!("{}/{}: symlink failed: {}", tool, name, e));
                continue;
            }

            if !path.join("SKILL.md").exists() {
                let _ = std::fs::remove_file(&path);
                let _ = std::fs::rename(&bak_path, &path);
                let _ = std::fs::remove_dir_all(&hub_target);
                report.errors.push(format!("{}/{}: symlink verification failed", tool, name));
                continue;
            }

            let _ = std::fs::remove_dir_all(&bak_path);
            report.imported.push((*tool, name));
        }
    }

    Ok(report)
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ── migrate ──

/// 复制 skillshub 到新路径，重建所有 symlink，校验一致性
pub fn migrate(old_path: &Path, new_path: &Path, config: &AppConfig) -> Result<MigrateReport, String> {
    if !old_path.exists() {
        return Err(format!("Old skillshub path does not exist: {}", old_path.display()));
    }
    if new_path.exists() && std::fs::read_dir(new_path).map(|mut d| d.next().is_some()).unwrap_or(false) {
        return Err(format!("New path already exists and is not empty: {}", new_path.display()));
    }

    let mut report = MigrateReport {
        copied: Vec::new(),
        symlinks_updated: Vec::new(),
        errors: Vec::new(),
        verified: false,
    };

    // 1. 复制所有 skill 目录到新路径
    std::fs::create_dir_all(new_path).map_err(|e| e.to_string())?;
    let entries = std::fs::read_dir(old_path).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let src = entry.path();
        if !src.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let dst = new_path.join(&name);
        match copy_dir_recursive(&src, &dst) {
            Ok(_) => report.copied.push(name),
            Err(e) => report.errors.push(format!("Copy {}: {}", name, e)),
        }
    }

    // 2. 重建所有 symlink
    let old_canonical = old_path.canonicalize().unwrap_or_else(|_| old_path.to_path_buf());
    for tool in &ALL_TOOLS {
        let tool_dir = match config.get_skills_dir(tool) {
            Some(d) if d.exists() => d,
            _ => continue,
        };
        let entries = match std::fs::read_dir(&tool_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_symlink() {
                continue;
            }
            let target = match std::fs::read_link(&path) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let target_canonical = target.canonicalize()
                .unwrap_or_else(|_| target.clone());
            if !target_canonical.starts_with(&old_canonical) {
                continue;
            }
            let skill_name = match target_canonical.strip_prefix(&old_canonical) {
                Ok(rel) => match rel.components().next() {
                    Some(c) => c.as_os_str().to_string_lossy().to_string(),
                    None => continue,
                },
                Err(_) => continue,
            };
            let new_target = new_path.join(&skill_name);
            if std::fs::remove_file(&path).is_ok() {
                match std::os::unix::fs::symlink(&new_target, &path) {
                    Ok(_) => report.symlinks_updated.push((*tool, skill_name)),
                    Err(e) => report.errors.push(format!(
                        "Symlink {}/{}: {}",
                        tool,
                        entry.file_name().to_string_lossy(),
                        e
                    )),
                }
            }
        }
    }

    // 3. 校验
    let mut verify_config = config.clone();
    if let Some(hub) = verify_config.tools.get_mut("skillshub") {
        hub.skills_dir = new_path.to_string_lossy().to_string();
    }
    report.verified = match verify(&verify_config) {
        Ok(v) => v.broken.is_empty(),
        Err(_) => false,
    };

    // 校验通过后删除旧目录
    if report.verified && report.errors.is_empty() {
        let _ = std::fs::remove_dir_all(old_path);
    }

    Ok(report)
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

    // ── verify tests ──

    #[test]
    fn verify_reports_healthy_symlinks() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("my-skill")).unwrap();
        fs::write(hub.join("my-skill/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        enable("my-skill", &Tool::ClaudeCode, &config).unwrap();

        let report = verify(&config).unwrap();
        assert_eq!(report.ok.len(), 1);
        assert_eq!(report.broken.len(), 0);
    }

    #[test]
    fn verify_detects_broken_symlink() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(&hub).unwrap();
        fs::create_dir_all(&tool).unwrap();

        let ghost = tmp.path().join("ghost");
        std::os::unix::fs::symlink(&ghost, tool.join("broken")).unwrap();

        let config = test_config(&hub, &tool);
        let report = verify(&config).unwrap();
        assert_eq!(report.broken.len(), 1);
    }

    // ── sync tests ──

    #[test]
    fn sync_imports_non_symlink_skill() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(&hub).unwrap();
        fs::create_dir_all(tool.join("new-skill")).unwrap();
        fs::write(tool.join("new-skill/SKILL.md"), "---\nname: new-skill\n---\n").unwrap();

        let config = test_config(&hub, &tool);
        let report = sync(&config).unwrap();

        assert_eq!(report.imported.len(), 1);
        assert_eq!(report.imported[0].1, "new-skill");
        assert!(tool.join("new-skill").is_symlink());
        assert!(hub.join("new-skill/SKILL.md").exists());
    }

    #[test]
    fn sync_skips_name_collision() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("dup")).unwrap();
        fs::write(hub.join("dup/SKILL.md"), "old").unwrap();
        fs::create_dir_all(tool.join("dup")).unwrap();
        fs::write(tool.join("dup/SKILL.md"), "new").unwrap();

        let config = test_config(&hub, &tool);
        let report = sync(&config).unwrap();

        assert_eq!(report.skipped.len(), 1);
        assert!(!tool.join("dup").is_symlink());
    }

    #[test]
    fn sync_ignores_symlinks() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("existing")).unwrap();
        fs::write(hub.join("existing/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();
        std::os::unix::fs::symlink(hub.join("existing"), tool.join("existing")).unwrap();

        let config = test_config(&hub, &tool);
        let report = sync(&config).unwrap();

        assert_eq!(report.imported.len(), 0);
        assert_eq!(report.skipped.len(), 0);
    }

    // ── migrate tests ──

    #[test]
    fn migrate_copies_and_rebuilds_symlinks() {
        let tmp = TempDir::new().unwrap();
        let old_hub = tmp.path().join("old-hub");
        let new_hub = tmp.path().join("new-hub");
        let tool = tmp.path().join("tool");

        fs::create_dir_all(old_hub.join("skill-a")).unwrap();
        fs::write(old_hub.join("skill-a/SKILL.md"), "content-a").unwrap();
        fs::create_dir_all(&tool).unwrap();
        std::os::unix::fs::symlink(old_hub.join("skill-a"), tool.join("skill-a")).unwrap();

        let config = test_config(&old_hub, &tool);
        let report = migrate(&old_hub, &new_hub, &config).unwrap();

        assert_eq!(report.copied, vec!["skill-a"]);
        assert_eq!(report.symlinks_updated.len(), 1);
        assert!(report.verified);
        assert!(new_hub.join("skill-a/SKILL.md").exists());
        let target = fs::read_link(tool.join("skill-a")).unwrap();
        assert_eq!(target, new_hub.join("skill-a"));
    }

    #[test]
    fn migrate_rejects_nonempty_target() {
        let tmp = TempDir::new().unwrap();
        let old_hub = tmp.path().join("old");
        let new_hub = tmp.path().join("new");
        fs::create_dir_all(old_hub.join("skill")).unwrap();
        fs::create_dir_all(new_hub.join("something")).unwrap();
        fs::write(new_hub.join("something/file"), "data").unwrap();

        let config = test_config(&old_hub, &tmp.path().join("tool"));
        let result = migrate(&old_hub, &new_hub, &config);
        assert!(result.is_err());
    }
}
