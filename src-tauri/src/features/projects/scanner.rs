/**
 * [INPUT]: 依赖 serde_json, chrono, super::types::SessionMeta
 * [OUTPUT]: 对外提供 parse_session_meta() 会话 JSONL 解析
 * [POS]: projects 的会话扫描器，从 JSONL 提取标题/计数/时间戳
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::SessionMeta;
use chrono::DateTime;
use serde::Deserialize;

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
}
