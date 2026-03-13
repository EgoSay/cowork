/**
 * [INPUT]: 依赖 ToolScanner trait, shared/fs_utils, types, serde_yaml
 * [OUTPUT]: 对外提供 ClaudeCodeScanner (扫描 ~/.claude/skills/)
 * [POS]: scanner/ 的 Claude Code 实现，解析 SKILL.md YAML frontmatter
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::ToolScanner;
use crate::features::skills::types::SkillMeta;
use crate::shared::fs_utils::{file_modified_at, hash_content, path_to_id};
use crate::types::{SkillFormat, Status, Tool};
use std::path::Path;

pub struct ClaudeCodeScanner;

impl ToolScanner for ClaudeCodeScanner {
    fn tool() -> Tool { Tool::ClaudeCode }

    fn scan(dir: &Path, _patterns: &[String]) -> Vec<SkillMeta> {
        let mut results = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else { return results };

        for entry in entries.flatten() {
            let path = entry.path();

            // 目录：查找 SKILL.md
            if path.is_dir() {
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    if let Some(meta) = parse_skill_md(&skill_md, Tool::ClaudeCode) {
                        results.push(meta);
                    }
                }
            }

            // 符号链接：解析后查找 SKILL.md
            if path.is_symlink() {
                if let Ok(resolved) = std::fs::read_link(&path) {
                    let target = if resolved.is_absolute() {
                        resolved
                    } else {
                        dir.join(&resolved)
                    };
                    let skill_md = target.join("SKILL.md");
                    if skill_md.exists() {
                        if let Some(meta) = parse_skill_md(&skill_md, Tool::ClaudeCode) {
                            results.push(meta);
                        }
                    }
                }
            }
        }

        results
    }
}

/// 解析 SKILL.md 的 YAML frontmatter，tool 参数决定 source_tool 归属
pub(crate) fn parse_skill_md(path: &Path, tool: Tool) -> Option<SkillMeta> {
    let content = std::fs::read_to_string(path).ok()?;
    let content_hash = hash_content(content.as_bytes());

    let (name, description, version) = if content.starts_with("---") {
        if let Some(end) = content[3..].find("---") {
            let yaml_str = &content[3..3 + end];
            let yaml: serde_yaml::Value = serde_yaml::from_str(yaml_str).ok()?;
            let name = yaml.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| path.parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown"))
                .to_string();
            let description = yaml.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let version = yaml.get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (name, description, version)
        } else {
            return None;
        }
    } else {
        // 无 frontmatter，用目录名
        let name = path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        (name, String::new(), None)
    };

    let modified_at = file_modified_at(path)
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    Some(SkillMeta {
        id: path_to_id(path),
        name,
        description,
        source_tool: tool,
        file_path: path.to_string_lossy().to_string(),
        format: SkillFormat::SkillMd,
        status: Status::Active,
        version,
        modified_at,
        content_hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parse_skill_md_with_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let skill_path = skill_dir.join("SKILL.md");
        fs::write(&skill_path, "---\nname: my-skill\ndescription: Does stuff\nversion: \"1.0\"\n---\n# Content here").unwrap();

        let meta = parse_skill_md(&skill_path, Tool::ClaudeCode).unwrap();
        assert_eq!(meta.name, "my-skill");
        assert_eq!(meta.description, "Does stuff");
        assert_eq!(meta.version, Some("1.0".into()));
        assert_eq!(meta.source_tool, Tool::ClaudeCode);
        assert_eq!(meta.format, SkillFormat::SkillMd);
        assert_eq!(meta.status, Status::Active);
        assert!(!meta.content_hash.is_empty());
        assert_eq!(meta.id.len(), 16);
    }

    #[test]
    fn parse_skill_md_without_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("fallback-name");
        fs::create_dir_all(&skill_dir).unwrap();
        let skill_path = skill_dir.join("SKILL.md");
        fs::write(&skill_path, "Just plain content, no YAML").unwrap();

        let meta = parse_skill_md(&skill_path, Tool::ClaudeCode).unwrap();
        assert_eq!(meta.name, "fallback-name"); // 用目录名
        assert!(meta.description.is_empty());
        assert!(meta.version.is_none());
    }

    #[test]
    fn parse_skill_md_broken_yaml_returns_none() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("broken");
        fs::create_dir_all(&skill_dir).unwrap();
        let skill_path = skill_dir.join("SKILL.md");
        // 有 --- 开头但 YAML 不闭合
        fs::write(&skill_path, "---\ninvalid yaml without closing").unwrap();

        assert!(parse_skill_md(&skill_path, Tool::ClaudeCode).is_none());
    }

    #[test]
    fn scan_finds_skills_in_subdirs() {
        let tmp = TempDir::new().unwrap();

        let s1 = tmp.path().join("skill-1");
        fs::create_dir_all(&s1).unwrap();
        fs::write(s1.join("SKILL.md"), "---\nname: one\ndescription: first\n---\n").unwrap();

        let s2 = tmp.path().join("skill-2");
        fs::create_dir_all(&s2).unwrap();
        fs::write(s2.join("SKILL.md"), "---\nname: two\ndescription: second\n---\n").unwrap();

        let results = ClaudeCodeScanner::scan(tmp.path(), &[]);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn scan_ignores_dirs_without_skill_md() {
        let tmp = TempDir::new().unwrap();
        let empty_dir = tmp.path().join("no-skill");
        fs::create_dir_all(&empty_dir).unwrap();
        fs::write(empty_dir.join("README.md"), "Not a skill").unwrap();

        let results = ClaudeCodeScanner::scan(tmp.path(), &[]);
        assert!(results.is_empty());
    }
}
