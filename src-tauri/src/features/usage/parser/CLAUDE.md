# features/usage/parser/
> L2 | Parent: src-tauri/src/features/usage/

Session JSONL parsers with unified token accounting. LOOKBACK_DAYS 定义在 mod.rs（单一真相源）。

## Members
- `mod.rs`: LOOKBACK_DAYS 常量（单一真相源）, TokenBucket/Accum 共享类型, parse_all() coordinator — defines scan window via scan_window_dates(), merges Claude + Codex records, clips events outside [scanned_from, scanned_until]; shared timestamp_to_date
- `claude_code.rs`: scans ~/.claude/projects/**/*.jsonl (含 subagents, mtime via super::LOOKBACK_DAYS), dedup by message.id, sums per (date, model)
- `codex.rs`: glob+mtime 扫描 ~/.codex/sessions/**/*.jsonl (mtime via super::LOOKBACK_DAYS), incremental last_token_usage + event timestamp 做日归属

## Token Accounting
Claude: 4 independent fields from API → direct mapping
Codex: cached_input ⊂ input → subtract to normalize: input = api.input - api.cached

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
