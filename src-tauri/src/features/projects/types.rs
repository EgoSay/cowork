/**
 * [INPUT]: 依赖 serde 序列化, std::collections::HashMap
 * [OUTPUT]: 对外提供 ProjectMeta, SessionMeta, SessionAnnotation, SessionMessage, ProjectData, CacheEntry, ProjectsCache
 * [POS]: projects 功能的核心数据类型，被 scanner/annotations/commands 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── 项目元信息 ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub id: String,
    pub name: String,
    pub dir_name: String,
    pub dir_path: String,
    pub session_count: usize,
    pub last_active: String,
    pub total_sessions_duration_secs: i64,
}

// ── 会话元信息 ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_secs: i64,
    pub message_count: usize,
    pub user_message_count: usize,
    pub turn_count: usize,
    pub has_subagents: bool,
}

// ── 会话标注 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAnnotation {
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: String,
}

// ── 会话消息（详情页用） ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    #[serde(rename = "type")]
    pub msg_type: String, // "system" | "user" | "assistant"
    pub content: String,  // message content text
    pub timestamp: String, // ISO timestamp
}

// ── 项目详情（含会话列表） ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectData {
    pub project: ProjectMeta,
    pub sessions: Vec<SessionMeta>,
}

// ── 缓存结构 ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub mtime_secs: i64,
    pub meta: SessionMeta,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectsCache {
    pub entries: HashMap<String, CacheEntry>,
}

// ── 测试 ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_session_meta() -> SessionMeta {
        SessionMeta {
            id: "sess-001".into(),
            project_id: "proj-001".into(),
            title: "implement scanner".into(),
            started_at: "2026-03-11T14:00:00+08:00".into(),
            ended_at: "2026-03-11T14:15:00+08:00".into(),
            duration_secs: 900,
            message_count: 7,
            user_message_count: 3,
            turn_count: 3,
            has_subagents: false,
        }
    }

    #[test]
    fn session_meta_serialization_roundtrip() {
        let meta = sample_session_meta();
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: SessionMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "sess-001");
        assert_eq!(parsed.duration_secs, 900);
        assert_eq!(parsed.turn_count, 3);
    }

    #[test]
    fn project_meta_serialization_roundtrip() {
        let meta = ProjectMeta {
            id: "proj-001".into(),
            name: "cowork".into(),
            dir_name: "cowork-abc123".into(),
            dir_path: "/Users/test/.claude/projects/cowork-abc123".into(),
            session_count: 5,
            last_active: "2026-03-11T14:15:00+08:00".into(),
            total_sessions_duration_secs: 3600,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: ProjectMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "cowork");
        assert_eq!(parsed.session_count, 5);
    }

    #[test]
    fn annotation_note_omitted_when_none() {
        let ann = SessionAnnotation {
            tags: vec!["refactor".into()],
            note: None,
            created_at: "2026-03-11T14:00:00+08:00".into(),
        };
        let json = serde_json::to_string(&ann).unwrap();
        assert!(!json.contains("note"));

        let ann_with_note = SessionAnnotation {
            tags: vec!["bug".into()],
            note: Some("critical fix".into()),
            created_at: "2026-03-11T14:00:00+08:00".into(),
        };
        let json = serde_json::to_string(&ann_with_note).unwrap();
        assert!(json.contains("critical fix"));
    }

    #[test]
    fn cache_roundtrip() {
        let mut cache = ProjectsCache::default();
        cache.entries.insert("sess-001".into(), CacheEntry {
            mtime_secs: 1710144000,
            meta: sample_session_meta(),
        });
        let json = serde_json::to_string(&cache).unwrap();
        let parsed: ProjectsCache = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries["sess-001"].mtime_secs, 1710144000);
        assert_eq!(parsed.entries["sess-001"].meta.title, "implement scanner");
    }
}
