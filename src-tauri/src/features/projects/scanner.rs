/**
 * [INPUT]: 依赖 serde_json, chrono, types::{SessionMeta,SessionMessage,ProjectMeta,ProjectData,ProjectsCache,CacheEntry}, shared::fs_utils::path_to_id
 * [OUTPUT]: 对外提供 parse_session_meta(), parse_session_messages(), extract_project_name(), scan_from_dir(), scan_all()
 * [POS]: projects 的核心扫描器——JSONL 解析 + 目录遍历 + 缓存加速
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::{CacheEntry, ProjectData, ProjectMeta, ProjectsCache, SessionMessage, SessionMeta};
use crate::shared::fs_utils::path_to_id;
use chrono::DateTime;
use serde::Deserialize;
use std::path::{Path, PathBuf};

// ── JSONL 事件结构 ───────────────────────────────────────

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "type")]
    event_type: String,
    message: Option<serde_json::Value>,
    timestamp: Option<String>,
}

// ── 标题提取 ─────────────────────────────────────────────

fn extract_title(value: &serde_json::Value) -> Option<String> {
    let raw = value.get("content")?.as_str()?;
    if raw.is_empty() {
        return None;
    }
    let limit = 120;
    if raw.len() <= limit {
        Some(raw.to_string())
    } else {
        // 手动实现 floor_char_boundary（避免 nightly 依赖）
        let mut boundary = limit;
        while boundary > 0 && !raw.is_char_boundary(boundary) {
            boundary -= 1;
        }
        Some(format!("{}...", &raw[..boundary]))
    }
}

// ── 核心解析 ─────────────────────────────────────────────

pub fn parse_session_meta(content: &str, session_id: &str, project_id: &str) -> Option<SessionMeta> {
    let mut message_count: usize = 0;
    let mut user_message_count: usize = 0;
    let mut assistant_count: usize = 0;
    let mut title: Option<String> = None;
    let mut first_ts: Option<DateTime<chrono::FixedOffset>> = None;
    let mut last_ts: Option<DateTime<chrono::FixedOffset>> = None;

    for line in content.lines() {
        let event: Event = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // 解析时间戳
        if let Some(ref ts_str) = event.timestamp {
            if let Ok(ts) = DateTime::parse_from_rfc3339(ts_str) {
                if first_ts.is_none() || ts < first_ts.unwrap() {
                    first_ts = Some(ts);
                }
                if last_ts.is_none() || ts > last_ts.unwrap() {
                    last_ts = Some(ts);
                }
            }
        }

        match event.event_type.as_str() {
            "user" => {
                message_count += 1;
                user_message_count += 1;
                if title.is_none() {
                    if let Some(ref msg) = event.message {
                        title = extract_title(msg);
                    }
                }
            }
            "assistant" => {
                message_count += 1;
                assistant_count += 1;
            }
            _ => {
                message_count += 1;
            }
        }
    }

    if message_count == 0 {
        return None;
    }

    let started_at = first_ts.map(|t| t.to_rfc3339()).unwrap_or_default();
    let ended_at = last_ts.map(|t| t.to_rfc3339()).unwrap_or_default();

    let duration_secs = match (first_ts, last_ts) {
        (Some(s), Some(e)) => (e - s).num_seconds().max(0),
        _ => 0,
    };

    let turn_count = user_message_count.min(assistant_count);
    let final_title = title.unwrap_or_else(|| "(no user message)".into());

    Some(SessionMeta {
        id: session_id.to_string(),
        project_id: project_id.to_string(),
        title: final_title,
        started_at,
        ended_at,
        duration_secs,
        message_count,
        user_message_count,
        turn_count,
        has_subagents: false,
    })
}

// ── 项目名提取 ───────────────────────────────────────────

/// 从编码后的目录名提取可读的项目名
/// 编码规则: `/` → `-`, `.` → `-`
/// 策略: 拆分后取最后两段（若均 ≤10 字符），否则只取最后一段
pub fn extract_project_name(dir_name: &str) -> String {
    let segments: Vec<&str> = dir_name.split('-').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return dir_name.to_string();
    }
    if segments.len() >= 2 {
        let last = segments[segments.len() - 1];
        let second_last = segments[segments.len() - 2];
        if last.len() <= 10 && second_last.len() <= 10 {
            return format!("{}-{}", second_last, last);
        }
    }
    segments[segments.len() - 1].to_string()
}

// ── 缓存操作 ─────────────────────────────────────────────

fn cache_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cowork")
        .join("projects_cache.json")
}

fn load_cache() -> ProjectsCache {
    let path = cache_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn save_cache(cache: &ProjectsCache) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = std::fs::write(&path, json);
    }
}

// ── 可测试的缓存操作（接受显式路径） ─────────────────────

fn load_cache_from(path: &Path) -> ProjectsCache {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn save_cache_to(path: &Path, cache: &ProjectsCache) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = std::fs::write(path, json);
    }
}

// ── 获取文件 mtime（秒级精度） ───────────────────────────

fn file_mtime_secs(path: &Path) -> Option<i64> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let dur = mtime.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(dur.as_secs() as i64)
}

// ── 目录扫描核心 ─────────────────────────────────────────

/// 从指定目录扫描项目数据（可测试版本，接受缓存路径）
pub fn scan_from_dir_with_cache(base: &Path, cache_path: &Path) -> Vec<ProjectData> {
    let mut cache = load_cache_from(cache_path);
    let mut results: Vec<ProjectData> = Vec::new();

    let entries = match std::fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        let proj_path = entry.path();
        if !proj_path.is_dir() {
            continue;
        }

        let dir_name = match proj_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let project_id = path_to_id(&proj_path);
        let project_name = extract_project_name(&dir_name);

        // 收集直接子目录中的 *.jsonl 文件（排除 subagents/）
        let mut sessions: Vec<SessionMeta> = Vec::new();

        let files = match std::fs::read_dir(&proj_path) {
            Ok(f) => f,
            Err(_) => continue,
        };

        for file_entry in files.flatten() {
            let file_path = file_entry.path();

            // 只处理直接子文件（非目录）
            if !file_path.is_file() {
                continue;
            }

            // 只处理 .jsonl 扩展名
            let ext = file_path.extension().and_then(|e| e.to_str());
            if ext != Some("jsonl") {
                continue;
            }

            let file_key = file_path.to_string_lossy().to_string();
            let mtime = file_mtime_secs(&file_path).unwrap_or(0);

            // 检查缓存
            if let Some(cached) = cache.entries.get(&file_key) {
                if cached.mtime_secs == mtime {
                    let mut meta = cached.meta.clone();
                    meta.project_id = project_id.clone();
                    sessions.push(meta);
                    continue;
                }
            }

            // 缓存未命中，解析 JSONL
            let content = match std::fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let stem = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            if let Some(mut meta) = parse_session_meta(&content, stem, &project_id) {
                // 检查 {stem}/subagents/ 目录
                let subagents_dir = proj_path.join(stem).join("subagents");
                meta.has_subagents = subagents_dir.is_dir();

                // 更新缓存
                cache.entries.insert(
                    file_key,
                    CacheEntry {
                        mtime_secs: mtime,
                        meta: meta.clone(),
                    },
                );

                sessions.push(meta);
            }
        }

        if sessions.is_empty() {
            continue;
        }

        // 聚合项目元信息
        let session_count = sessions.len();
        let last_active = sessions
            .iter()
            .map(|s| s.ended_at.as_str())
            .max()
            .unwrap_or("")
            .to_string();
        let total_duration: i64 = sessions.iter().map(|s| s.duration_secs).sum();

        // 按 ended_at 降序排列会话
        sessions.sort_by(|a, b| b.ended_at.cmp(&a.ended_at));

        results.push(ProjectData {
            project: ProjectMeta {
                id: project_id,
                name: project_name,
                dir_name,
                dir_path: proj_path.to_string_lossy().to_string(),
                session_count,
                last_active,
                total_sessions_duration_secs: total_duration,
            },
            sessions,
        });
    }

    // 按 last_active 降序排列
    results.sort_by(|a, b| b.project.last_active.cmp(&a.project.last_active));

    // 保存缓存
    save_cache_to(cache_path, &cache);

    results
}

/// 从指定目录扫描（使用全局缓存路径）
pub fn scan_from_dir(base: &Path) -> Vec<ProjectData> {
    scan_from_dir_with_cache(base, &cache_path())
}

/// 公共入口：扫描 ~/.claude/projects/
pub fn scan_all() -> Vec<ProjectData> {
    let base = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("projects");
    scan_from_dir(&base)
}

// ── 会话消息解析（详情页用） ─────────────────────────────

/// 解析 JSONL 文件的完整消息列表（用于会话详情页）
pub fn parse_session_messages(content: &str) -> Vec<SessionMessage> {
    let mut messages = Vec::new();
    for line in content.lines() {
        let event: Event = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let timestamp = event.timestamp.unwrap_or_default();

        let content = match &event.message {
            Some(msg) => msg
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            None => String::new(),
        };

        // Skip assistant messages with no meaningful content
        if content.is_empty() && event.event_type == "assistant" {
            continue;
        }

        messages.push(SessionMessage {
            msg_type: event.event_type,
            content,
            timestamp,
        });
    }
    messages
}

// ── 测试 ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const MOCK_SESSION: &str = concat!(
        r#"{"type":"system","message":{"content":"system prompt"},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
        r#"{"type":"user","message":{"content":"implement the project scanner for cowork"},"timestamp":"2026-03-11T14:01:00+08:00"}"#, "\n",
        r#"{"type":"assistant","message":{"id":"msg_01","model":"claude-opus-4-6"},"timestamp":"2026-03-11T14:02:00+08:00"}"#, "\n",
        r#"{"type":"user","message":{"content":"looks good, now add tests"},"timestamp":"2026-03-11T14:05:00+08:00"}"#, "\n",
        r#"{"type":"assistant","message":{"id":"msg_02","model":"claude-opus-4-6"},"timestamp":"2026-03-11T14:10:00+08:00"}"#, "\n",
        r#"{"type":"user","message":{"content":"perfect, ship it"},"timestamp":"2026-03-11T14:12:00+08:00"}"#, "\n",
        r#"{"type":"assistant","message":{"id":"msg_03","model":"claude-opus-4-6"},"timestamp":"2026-03-11T14:15:00+08:00"}"#
    );

    const MOCK_SESSION_2: &str = concat!(
        r#"{"type":"user","message":{"content":"second session"},"timestamp":"2026-03-12T10:00:00+08:00"}"#, "\n",
        r#"{"type":"assistant","message":{"id":"msg_04","model":"claude-opus-4-6"},"timestamp":"2026-03-12T10:05:00+08:00"}"#
    );

    // ── parse_session_meta 测试 ──────────────────────────

    #[test]
    fn parse_extracts_title_from_first_user_message() {
        let meta = parse_session_meta(MOCK_SESSION, "sess-1", "proj-1").unwrap();
        assert_eq!(meta.title, "implement the project scanner for cowork");
    }

    #[test]
    fn parse_counts_messages_correctly() {
        let meta = parse_session_meta(MOCK_SESSION, "sess-1", "proj-1").unwrap();
        assert_eq!(meta.message_count, 7);       // 1 system + 3 user + 3 assistant
        assert_eq!(meta.user_message_count, 3);
        assert_eq!(meta.turn_count, 3);           // min(3 user, 3 assistant)
    }

    #[test]
    fn parse_extracts_timestamps() {
        let meta = parse_session_meta(MOCK_SESSION, "sess-1", "proj-1").unwrap();
        assert!(meta.started_at.contains("14:00:00"));
        assert!(meta.ended_at.contains("14:15:00"));
        assert_eq!(meta.duration_secs, 900);      // 15 minutes
    }

    #[test]
    fn parse_truncates_long_title() {
        let long_msg = "x".repeat(200);
        let content = format!(
            r#"{{"type":"user","message":{{"content":"{}"}},"timestamp":"2026-03-11T14:00:00+08:00"}}"#,
            long_msg
        );
        let meta = parse_session_meta(&content, "sess-1", "proj-1").unwrap();
        assert!(meta.title.len() <= 123); // 120 + "..."
        assert!(meta.title.ends_with("..."));
    }

    #[test]
    fn parse_empty_returns_none() {
        assert!(parse_session_meta("", "sess-1", "proj-1").is_none());
        assert!(parse_session_meta("garbage line\nnot json", "sess-1", "proj-1").is_none());
    }

    #[test]
    fn parse_no_user_message_uses_fallback_title() {
        let content = concat!(
            r#"{"type":"system","message":{"content":"init"},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"msg_01","model":"claude-opus-4-6"},"timestamp":"2026-03-11T14:01:00+08:00"}"#
        );
        let meta = parse_session_meta(content, "sess-1", "proj-1").unwrap();
        assert_eq!(meta.title, "(no user message)");
        assert_eq!(meta.user_message_count, 0);
    }

    // ── extract_project_name 测试 ────────────────────────

    #[test]
    fn scan_extracts_project_name_last_segment() {
        // 典型 Claude 项目目录名：最后两段 feat + project 均 ≤10 字符
        let name = extract_project_name("-Users-adairchan--superset-worktrees-cowork-feat-project");
        assert_eq!(name, "feat-project");
    }

    #[test]
    fn extract_name_single_segment() {
        assert_eq!(extract_project_name("myproject"), "myproject");
    }

    #[test]
    fn extract_name_long_segment_takes_last_only() {
        // 第二段超过 10 字符，只取最后一段
        let name = extract_project_name("very-longprojectname-short");
        assert_eq!(name, "short");
    }

    #[test]
    fn extract_name_both_short_takes_two() {
        assert_eq!(extract_project_name("foo-bar-baz"), "bar-baz");
    }

    #[test]
    fn extract_name_empty_returns_original() {
        assert_eq!(extract_project_name(""), "");
    }

    // ── scan_from_dir 测试 ───────────────────────────────

    #[test]
    fn scan_projects_dir_builds_project_data() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        // 创建项目目录
        let proj_dir = base.join("my-project");
        std::fs::create_dir_all(&proj_dir).unwrap();

        // 写入两个 JSONL 文件
        std::fs::write(proj_dir.join("session-aaa.jsonl"), MOCK_SESSION).unwrap();
        std::fs::write(proj_dir.join("session-bbb.jsonl"), MOCK_SESSION_2).unwrap();

        let cache_file = tmp.path().join("test_cache.json");
        let results = scan_from_dir_with_cache(base, &cache_file);

        assert_eq!(results.len(), 1);
        let data = &results[0];
        assert_eq!(data.project.name, "my-project");
        assert_eq!(data.project.session_count, 2);
        assert_eq!(data.sessions.len(), 2);
        assert!(data.project.total_sessions_duration_secs > 0);
        assert!(!data.project.last_active.is_empty());
    }

    #[test]
    fn scan_excludes_subagent_files() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        let proj_dir = base.join("my-project");
        std::fs::create_dir_all(&proj_dir).unwrap();

        // 主会话文件
        std::fs::write(proj_dir.join("session-aaa.jsonl"), MOCK_SESSION).unwrap();

        // subagents 子目录（不应被扫描为会话，但应标记 has_subagents）
        let subagent_dir = proj_dir.join("session-aaa").join("subagents");
        std::fs::create_dir_all(&subagent_dir).unwrap();
        std::fs::write(subagent_dir.join("sub-001.jsonl"), MOCK_SESSION_2).unwrap();

        let cache_file = tmp.path().join("test_cache.json");
        let results = scan_from_dir_with_cache(base, &cache_file);

        assert_eq!(results.len(), 1);
        let data = &results[0];
        // subagent 文件不计入 session
        assert_eq!(data.project.session_count, 1);
        assert_eq!(data.sessions.len(), 1);
        // 但标记 has_subagents
        assert!(data.sessions[0].has_subagents);
    }

    #[test]
    fn scan_cache_accelerates_second_run() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        let proj_dir = base.join("cached-project");
        std::fs::create_dir_all(&proj_dir).unwrap();
        std::fs::write(proj_dir.join("sess-1.jsonl"), MOCK_SESSION).unwrap();

        let cache_file = tmp.path().join("test_cache.json");

        // 第一次扫描：填充缓存
        let r1 = scan_from_dir_with_cache(base, &cache_file);
        assert_eq!(r1.len(), 1);

        // 缓存文件应存在
        assert!(cache_file.exists());

        // 第二次扫描：应该命中缓存
        let r2 = scan_from_dir_with_cache(base, &cache_file);
        assert_eq!(r2.len(), 1);
        assert_eq!(r2[0].sessions[0].title, r1[0].sessions[0].title);
    }

    #[test]
    fn scan_empty_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_file = tmp.path().join("test_cache.json");
        let results = scan_from_dir_with_cache(tmp.path(), &cache_file);
        assert!(results.is_empty());
    }

    // ── parse_session_messages 测试 ──────────────────

    #[test]
    fn parse_messages_extracts_all_types() {
        let messages = parse_session_messages(MOCK_SESSION);
        assert!(messages.len() >= 4); // system + 3 user + some assistant
        assert_eq!(messages[0].msg_type, "system");
        assert_eq!(messages[1].msg_type, "user");
        assert_eq!(messages[1].content, "implement the project scanner for cowork");
    }

    #[test]
    fn parse_messages_skips_empty_assistant() {
        // assistant messages in MOCK_SESSION have no "content" text field → should be skipped
        let messages = parse_session_messages(MOCK_SESSION);
        let assistant_msgs: Vec<_> = messages.iter().filter(|m| m.msg_type == "assistant").collect();
        assert!(assistant_msgs.is_empty());
    }

    #[test]
    fn parse_messages_empty_input() {
        let messages = parse_session_messages("");
        assert!(messages.is_empty());
    }
}
