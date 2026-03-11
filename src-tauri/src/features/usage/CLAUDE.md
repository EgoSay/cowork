# features/usage/
> L2 | Parent: src-tauri/src/features/

Token usage data aggregation. Both parsers output unified DailyRecord (4-field breakdown).

## Members
- `mod.rs`: module entry
- `types.rs`: DailyRecord (统一口径: input/output/cache_read/cache_write), UsageData
- `commands.rs`: get_usage_data Tauri IPC command
- `parser/`: dual-tool log parser (see parser/CLAUDE.md)

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
