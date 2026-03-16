/**
 * [INPUT]: 依赖 serde/toml 序列化, chrono 时间戳, types::SessionAnnotation
 * [OUTPUT]: 对外提供 load(), save(), upsert(), remove() 标注 CRUD
 * [POS]: projects 的标注子系统，管理 tags/notes 到 ~/.cowork/annotations.toml 的持久化
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::SessionAnnotation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── 内部文件结构 ─────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Default)]
struct AnnotationsFile {
    #[serde(default)]
    sessions: HashMap<String, SessionAnnotation>,
}

// ── 路径 ─────────────────────────────────────────────────

fn annotations_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cowork")
        .join("annotations.toml")
}

// ── 可测试的核心操作（接受显式路径） ─────────────────────

pub fn load_from(path: &Path) -> HashMap<String, SessionAnnotation> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| toml::from_str::<AnnotationsFile>(&content).ok())
        .map(|f| f.sessions)
        .unwrap_or_default()
}

pub fn save_to(
    path: &Path,
    annotations: &HashMap<String, SessionAnnotation>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create dir: {e}"))?;
    }
    let file = AnnotationsFile {
        sessions: annotations.clone(),
    };
    let content = toml::to_string(&file).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(path, content).map_err(|e| format!("write: {e}"))
}

// ── 公共接口（使用默认路径） ─────────────────────────────

pub fn load() -> HashMap<String, SessionAnnotation> {
    load_from(&annotations_path())
}

pub fn save(annotations: &HashMap<String, SessionAnnotation>) -> Result<(), String> {
    save_to(&annotations_path(), annotations)
}

pub fn upsert(session_id: &str, tags: Vec<String>, note: Option<String>) -> Result<(), String> {
    let path = annotations_path();
    let mut map = load_from(&path);
    let created_at = map
        .get(session_id)
        .map(|existing| existing.created_at.clone())
        .unwrap_or_else(|| chrono::Utc::now().timestamp().to_string());
    map.insert(
        session_id.to_string(),
        SessionAnnotation {
            tags,
            note,
            created_at,
        },
    );
    save_to(&path, &map)
}

pub fn remove(session_id: &str) -> Result<(), String> {
    let path = annotations_path();
    let mut map = load_from(&path);
    map.remove(session_id);
    save_to(&path, &map)
}

// ── 测试 ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("annotations.toml");

        let mut map = HashMap::new();
        map.insert(
            "sess-001".to_string(),
            SessionAnnotation {
                tags: vec!["refactor".into(), "scanner".into()],
                note: Some("重构了扫描器".into()),
                created_at: "1710144000".into(),
            },
        );

        save_to(&path, &map).unwrap();
        let loaded = load_from(&path);

        assert_eq!(loaded.len(), 1);
        let ann = &loaded["sess-001"];
        assert_eq!(ann.tags, vec!["refactor", "scanner"]);
        assert_eq!(ann.note.as_deref(), Some("重构了扫描器"));
        assert_eq!(ann.created_at, "1710144000");
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.toml");
        let map = load_from(&path);
        assert!(map.is_empty());
    }

    #[test]
    fn update_existing_annotation() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("annotations.toml");

        // 初始写入
        let mut map = HashMap::new();
        map.insert(
            "sess-001".to_string(),
            SessionAnnotation {
                tags: vec!["bug".into()],
                note: None,
                created_at: "1710144000".into(),
            },
        );
        save_to(&path, &map).unwrap();

        // 覆盖 tags
        let mut map = load_from(&path);
        map.insert(
            "sess-001".to_string(),
            SessionAnnotation {
                tags: vec!["feature".into(), "ui".into()],
                note: Some("updated".into()),
                created_at: "1710144000".into(),
            },
        );
        save_to(&path, &map).unwrap();

        let loaded = load_from(&path);
        let ann = &loaded["sess-001"];
        assert_eq!(ann.tags, vec!["feature", "ui"]);
        assert_eq!(ann.note.as_deref(), Some("updated"));
    }

    #[test]
    fn remove_annotation() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("annotations.toml");

        let mut map = HashMap::new();
        map.insert(
            "sess-001".to_string(),
            SessionAnnotation {
                tags: vec!["temp".into()],
                note: None,
                created_at: "1710144000".into(),
            },
        );
        save_to(&path, &map).unwrap();

        // 删除
        let mut map = load_from(&path);
        map.remove("sess-001");
        save_to(&path, &map).unwrap();

        let loaded = load_from(&path);
        assert!(loaded.is_empty());
    }
}
