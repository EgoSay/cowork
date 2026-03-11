/**
 * [INPUT]: 依赖 serde_json, chrono, glob, dirs, crate::types::Tool, super::{types, timestamp_to_date}
 * [OUTPUT]: 对外提供 parse() -> Vec<DailyRecord>
 * [POS]: Claude Code 会话解析器，读取 ~/.claude/projects/**/*.jsonl (含 subagents)
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::super::types::DailyRecord;
use super::timestamp_to_date;
use crate::types::Tool;
use chrono::{Local, TimeZone};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const LOOKBACK_DAYS: u64 = 31;

// ── session JSONL 事件结构 ──────────────────────────────

#[derive(Deserialize)]
struct SessionEvent {
    #[serde(rename = "type")]
    event_type: String,
    message: Option<serde_json::Value>,
    timestamp: Option<String>,
}

#[derive(Deserialize)]
struct AssistantMessage {
    id: Option<String>,
    model: Option<String>,
    usage: Option<ClaudeUsage>,
}

#[derive(Deserialize, Clone)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(rename = "cache_creation_input_tokens", default)]
    cache_creation_input_tokens: u64,
    #[serde(rename = "cache_read_input_tokens", default)]
    cache_read_input_tokens: u64,
}

// ── 核心：解析单个 session 文件 ────────────────────────
// 同一个 message.id 会连续出现 2-10 次（流式中间态），
// 只有最后一条携带完整 usage。先按 id 保留最后一条，
// 再做 (date, model) 聚合。

type Accum = HashMap<(String, String), (u64, u64, u64, u64)>;

// 中间结构: 按 message.id 保留最后一条
struct MessageSnapshot {
    date: String,
    model: String,
    usage: ClaudeUsage,
}

fn parse_session_content<Tz: TimeZone>(content: &str, tz: &Tz) -> Accum
where
    Tz::Offset: std::fmt::Display,
{
    // Phase 1: 按 message.id 保留最后一条（流式去重）
    // 同一个 id 出现 2-10 次，前几条是中间态，最后一条有完整 usage
    let mut snapshots: HashMap<String, MessageSnapshot> = HashMap::new();
    let mut anon_counter: usize = 0;

    for line in content.lines() {
        let event: SessionEvent = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if event.event_type != "assistant" { continue; }

        let msg: AssistantMessage = match event.message {
            Some(v) => match serde_json::from_value(v) {
                Ok(m) => m,
                Err(_) => continue,
            },
            None => continue,
        };

        let (model, usage) = match (msg.model, msg.usage) {
            (Some(m), Some(u)) => (m, u),
            _ => continue,
        };

        let date = event.timestamp
            .as_deref()
            .and_then(|ts| timestamp_to_date(ts, tz))
            .unwrap_or_default();
        if date.is_empty() { continue; }

        // 有 id 则按 id 去重（覆盖前一条）；无 id 则视为独立消息
        let key = match msg.id {
            Some(id) => id,
            None => {
                anon_counter += 1;
                format!("__anon_{}", anon_counter)
            }
        };

        snapshots.insert(key, MessageSnapshot { date, model, usage });
    }

    // Phase 2: 将去重后的快照聚合为 (date, model) → tokens
    let mut accum: Accum = HashMap::new();
    for snapshot in snapshots.into_values() {
        let entry = accum.entry((snapshot.date, snapshot.model)).or_default();
        entry.0 += snapshot.usage.input_tokens;
        entry.1 += snapshot.usage.output_tokens;
        entry.2 += snapshot.usage.cache_read_input_tokens;
        entry.3 += snapshot.usage.cache_creation_input_tokens;
    }

    accum
}

// ── 目录扫描 ────────────────────────────────────────────

fn projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("projects"))
}

fn parse_from_dir(base: &Path) -> Vec<DailyRecord> {
    // **/*.jsonl 递归匹配主会话 + subagents/
    let pattern = base.join("**").join("*.jsonl");
    let pattern_str = pattern.to_string_lossy().to_string();

    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(LOOKBACK_DAYS * 86400);

    let mut global: Accum = HashMap::new();

    let files = match glob::glob(&pattern_str) {
        Ok(paths) => paths,
        Err(_) => return vec![],
    };

    for entry in files.flatten() {
        // 跳过 31 天前的文件
        let dominated_by_mtime = entry.metadata()
            .and_then(|m| m.modified())
            .map_or(false, |mtime| mtime >= cutoff);
        if !dominated_by_mtime { continue; }

        let content = match std::fs::read_to_string(&entry) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for ((date, model), (inp, out, cr, cw)) in parse_session_content(&content, &Local) {
            let e = global.entry((date, model)).or_default();
            e.0 += inp;
            e.1 += out;
            e.2 += cr;
            e.3 += cw;
        }
    }

    global.into_iter()
        .map(|((date, model), (inp, out, cr, cw))| DailyRecord {
            date, tool: Tool::ClaudeCode, model,
            input_tokens: inp,
            output_tokens: out,
            cache_read_tokens: cr,
            cache_write_tokens: cw,
        })
        .collect()
}

pub fn parse() -> Vec<DailyRecord> {
    let base = match projects_dir() {
        Some(p) if p.exists() => p,
        _ => return vec![],
    };
    parse_from_dir(&base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::timestamp_to_date;
    use chrono::FixedOffset;
    use filetime;

    // ── 所有测试注入 +08:00 时区，确保确定性 ─────────────
    fn tz() -> FixedOffset {
        FixedOffset::east_opt(8 * 3600).unwrap()
    }

    const MOCK_SESSION: &str = concat!(
        r#"{"type":"system","message":{"content":"..."},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
        r#"{"type":"user","message":{"content":"hello"},"timestamp":"2026-03-11T14:01:00+08:00"}"#, "\n",
        r#"{"type":"assistant","message":{"id":"msg_01","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":2000,"cache_read_input_tokens":5000}},"timestamp":"2026-03-11T14:02:00+08:00"}"#, "\n",
        r#"{"type":"assistant","message":{"id":"msg_02","model":"claude-opus-4-6","usage":{"input_tokens":80,"output_tokens":30,"cache_creation_input_tokens":0,"cache_read_input_tokens":6000}},"timestamp":"2026-03-11T15:00:00+08:00"}"#
    );

    #[test]
    fn parse_session_aggregates_by_date_model() {
        let accum = parse_session_content(MOCK_SESSION, &tz());
        let key = ("2026-03-11".to_string(), "claude-opus-4-6".to_string());
        let (inp, out, cr, cw) = accum[&key];
        assert_eq!(inp, 180);     // 100 + 80
        assert_eq!(out, 80);      // 50 + 30
        assert_eq!(cr, 11000);    // 5000 + 6000
        assert_eq!(cw, 2000);     // 2000 + 0
    }

    #[test]
    fn parse_session_deduplicates_by_message_id() {
        // 同一个 message.id 出现 3 次（流式中间态 → 最终态）
        // output_tokens: 10 → 50 → 200，只取最后一条的 200
        let content = concat!(
            r#"{"type":"assistant","message":{"id":"msg_dup","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":0,"cache_creation_input_tokens":1744}},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"msg_dup","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":0,"cache_creation_input_tokens":1744}},"timestamp":"2026-03-11T14:00:01+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"msg_dup","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":200,"cache_read_input_tokens":5000,"cache_creation_input_tokens":1744}},"timestamp":"2026-03-11T14:00:02+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"msg_other","model":"claude-opus-4-6","usage":{"input_tokens":50,"output_tokens":30,"cache_read_input_tokens":3000,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T14:05:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        let key = ("2026-03-11".to_string(), "claude-opus-4-6".to_string());
        let (inp, out, _, _) = accum[&key];
        assert_eq!(out, 230);     // 200 (msg_dup final) + 30 (msg_other)
        assert_eq!(inp, 150);     // 100 (msg_dup final) + 50 (msg_other)
    }

    #[test]
    fn parse_session_handles_midnight_crossing() {
        // +08:00 下 23:30 和次日 00:30 跨天，注入时区后可断言精确日期
        let content = concat!(
            r#"{"type":"assistant","message":{"id":"msg_a","model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T23:30:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"msg_b","model":"claude-opus-4-6","usage":{"input_tokens":20,"output_tokens":10,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-12T00:30:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        assert_eq!(accum.len(), 2);
        // 精确断言日期桶
        let day1 = ("2026-03-11".to_string(), "claude-opus-4-6".to_string());
        let day2 = ("2026-03-12".to_string(), "claude-opus-4-6".to_string());
        assert_eq!(accum[&day1].0, 10);  // input
        assert_eq!(accum[&day1].1, 5);   // output
        assert_eq!(accum[&day2].0, 20);
        assert_eq!(accum[&day2].1, 10);
    }

    #[test]
    fn parse_session_no_id_still_counted() {
        let content = concat!(
            r#"{"type":"assistant","message":{"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":5}},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"model":"claude-opus-4-6","usage":{"input_tokens":20,"output_tokens":10}},"timestamp":"2026-03-11T14:01:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        let total_inp: u64 = accum.values().map(|v| v.0).sum();
        assert_eq!(total_inp, 30);
    }

    #[test]
    fn parse_session_skips_non_assistant_and_malformed() {
        let content = concat!(
            "garbage line\n",
            r#"{"type":"user","message":{"content":"hi"},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"msg_x","model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":5}},"timestamp":"2026-03-11T14:01:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        assert_eq!(accum.len(), 1);
        let (inp, out, cr, cw) = accum.values().next().unwrap();
        assert_eq!(*inp, 10);
        assert_eq!(*out, 5);
        assert_eq!(*cr, 0);
        assert_eq!(*cw, 0);
    }

    #[test]
    fn parse_session_empty_returns_empty() {
        let tz = tz();
        assert!(parse_session_content("", &tz).is_empty());
        assert!(parse_session_content("not json", &tz).is_empty());
    }

    #[test]
    fn parse_from_dir_scans_project_and_subagents() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project-abc");
        std::fs::create_dir_all(&project).unwrap();

        let main_session = r#"{"type":"assistant","message":{"id":"msg_main","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T14:00:00+08:00"}"#;
        std::fs::write(project.join("session-1.jsonl"), main_session).unwrap();

        let subagent_dir = project.join("session-1").join("subagents");
        std::fs::create_dir_all(&subagent_dir).unwrap();
        let sub_session = r#"{"type":"assistant","message":{"id":"msg_sub","model":"claude-haiku-4-5","usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T15:00:00+08:00"}"#;
        std::fs::write(subagent_dir.join("agent-abc.jsonl"), sub_session).unwrap();

        let records = parse_from_dir(dir.path());
        assert_eq!(records.len(), 2);
        assert!(records.iter().any(|r| r.model == "claude-opus-4-6"));
        assert!(records.iter().any(|r| r.model == "claude-haiku-4-5"));
    }

    #[test]
    fn parse_from_dir_skips_old_mtime_files() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project-old");
        std::fs::create_dir_all(&project).unwrap();

        let content = r#"{"type":"assistant","message":{"id":"msg_old","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2025-01-01T10:00:00+08:00"}"#;
        let old_file = project.join("session-old.jsonl");
        std::fs::write(&old_file, content).unwrap();

        // 强制 mtime 为 60 天前
        let old_time = filetime::FileTime::from_system_time(
            std::time::SystemTime::now() - std::time::Duration::from_secs(60 * 86400)
        );
        filetime::set_file_mtime(&old_file, old_time).unwrap();

        let records = parse_from_dir(dir.path());
        assert!(records.is_empty());
    }

    #[test]
    fn timestamp_to_date_with_fixed_offset() {
        let tz = FixedOffset::east_opt(8 * 3600).unwrap();
        assert_eq!(
            timestamp_to_date("2026-03-11T16:00:00Z", &tz).unwrap(),
            "2026-03-12"
        );
        assert_eq!(
            timestamp_to_date("2026-03-11T23:59:59+08:00", &tz).unwrap(),
            "2026-03-11"
        );
    }

    #[test]
    fn timestamp_to_date_fallback() {
        let tz = FixedOffset::east_opt(0).unwrap();
        assert_eq!(
            timestamp_to_date("2026-03-11T06:00:00", &tz),
            Some("2026-03-11".to_string())
        );
        assert_eq!(timestamp_to_date::<FixedOffset>("bad", &tz), None);
    }
}
