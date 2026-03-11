/**
 * [INPUT]: 依赖 serde_json, chrono, glob, dirs, crate::types::Tool, super::types
 * [OUTPUT]: 对外提供 parse() -> Vec<DailyRecord>
 * [POS]: Claude Code 会话解析器，读取 ~/.claude/projects/**/*.jsonl (含 subagents)
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::super::types::DailyRecord;

pub fn parse() -> Vec<DailyRecord> { vec![] }
