// [INPUT]: 依赖 ToolScanner trait, shared/fs_utils, glob
// [OUTPUT]: 对外提供 CursorScanner (扫描 ~/.cursor/rules/ 下 .mdc 文件)
// [POS]: scanner/ 的 Cursor 实现
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::ToolScanner;
use crate::features::skills::types::SkillMeta;
use crate::shared::fs_utils::{file_modified_at, hash_content, path_to_id};
use crate::types::{SkillFormat, Status, Tool};
use std::path::Path;

pub struct CursorScanner;

impl ToolScanner for CursorScanner {
    fn tool() -> Tool { Tool::Cursor }

    fn scan(dir: &Path, _patterns: &[String]) -> Vec<SkillMeta> {
        let mut results = Vec::new();
        let pattern = format!("{}/*.mdc", dir.display());
        if let Ok(entries) = glob::glob(&pattern) {
            for entry in entries.flatten() {
                if let Some(meta) = parse_mdc(&entry) {
                    results.push(meta);
                }
            }
        }
        results
    }
}

fn parse_mdc(path: &Path) -> Option<SkillMeta> {
    let content = std::fs::read_to_string(path).ok()?;
    let hash = hash_content(content.as_bytes());
    let name = path.file_stem()?.to_str()?.to_string();
    let desc = content.lines().take(3)
        .collect::<Vec<_>>().join(" ")
        .chars().take(120).collect::<String>();
    let modified_at = file_modified_at(path)
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    Some(SkillMeta {
        id: path_to_id(path),
        name,
        description: desc,
        source_tool: Tool::Cursor,
        file_path: path.to_string_lossy().to_string(),
        format: SkillFormat::CursorMdc,
        status: Status::Active,
        version: None,
        modified_at,
        content_hash: hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn scan_finds_mdc_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("my-rule.mdc"), "Rule content here\nLine 2").unwrap();
        fs::write(tmp.path().join("another.mdc"), "Another rule").unwrap();

        let results = CursorScanner::scan(tmp.path(), &[]);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.name == "my-rule"));
        assert!(results.iter().any(|r| r.name == "another"));
        assert!(results.iter().all(|r| r.source_tool == Tool::Cursor));
    }

    #[test]
    fn scan_ignores_non_mdc_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("readme.md"), "Not a rule").unwrap();
        fs::write(tmp.path().join("valid.mdc"), "A rule").unwrap();

        let results = CursorScanner::scan(tmp.path(), &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "valid");
    }

    #[test]
    fn description_truncated_to_120_chars() {
        let tmp = TempDir::new().unwrap();
        let long_line = "A".repeat(200);
        fs::write(tmp.path().join("long.mdc"), &long_line).unwrap();

        let results = CursorScanner::scan(tmp.path(), &[]);
        assert_eq!(results.len(), 1);
        assert!(results[0].description.len() <= 120);
    }
}
