# features/usage/
> L2 | Parent: src/features/

Token usage monitoring dashboard. Unified 4-field accounting (input/output/cache_read/cache_write).

## Members
- `lib.ts`: PresetRange/TimeRange/DateRange/ScanWindow types, formatTokens, localDateString, dateRange, clampToWindow, recordTotal
- `hooks/useUsage.ts`: single truth source from DailyRecord[], derives all aggregations via useMemo
- `pages/UsagePage.tsx`: main dashboard (summary cards, daily chart, model table with breakdown)
- `components/TimeRangeTab.tsx`: Today/7D/30D pill selector
- `components/SummaryCards.tsx`: 4-card grid (Total, Sent, Received, Cache Hit)
- `components/DailyChart.tsx`: CSS horizontal bar chart (Claude=text/80, Codex=text/30)
- `components/ModelTable.tsx`: model distribution table with input/output/cache columns

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
