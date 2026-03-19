# features/usage/
> L2 | Parent: src-tauri/src/features/

Token usage data aggregation. Both parsers output unified DailyRecord (4-field breakdown).

## Members
- `mod.rs`: module entry
- `types.rs`: DailyRecord (统一口径: input/output/cache_read/cache_write), UsageData (含 scanned_from/scanned_until 扫描窗口)
- `commands.rs`: get_usage_data Tauri IPC command (spawn_blocking)
- `parser/`: dual-tool log parser, LOOKBACK_DAYS 单一真相源, parse_all() defines scan window + clips events (see parser/CLAUDE.md)

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
