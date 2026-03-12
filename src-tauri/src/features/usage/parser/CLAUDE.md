# features/usage/parser/
> L2 | Parent: src-tauri/src/features/usage/

Session JSONL parsers with unified token accounting.

## Members
- `mod.rs`: parse_all() coordinator, merges Claude + Codex records; shared timestamp_to_date
- `claude_code.rs`: scans ~/.claude/projects/**/*.jsonl (含 subagents, mtime < 31d), dedup by message.id, sums per (date, model)
- `codex.rs`: glob+mtime 扫描 ~/.codex/sessions/**/*.jsonl, incremental last_token_usage + event timestamp 做日归属

## Token Accounting
Claude: 4 independent fields from API → direct mapping
Codex: cached_input ⊂ input → subtract to normalize: input = api.input - api.cached

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
