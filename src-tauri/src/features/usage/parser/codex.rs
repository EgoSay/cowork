/**
 * [INPUT]: 依赖 serde_json, chrono, dirs, crate::types::Tool, super::types
 * [OUTPUT]: 对外提供 parse() -> Vec<DailyRecord>
 * [POS]: Codex CLI 会话解析器，glob+mtime 扫描 ~/.codex/sessions/**/*.jsonl
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::super::types::DailyRecord;

pub fn parse() -> Vec<DailyRecord> { vec![] }
