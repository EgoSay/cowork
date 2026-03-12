# features/usage/
> L2 | Parent: src/features/

Token usage monitoring dashboard. Unified 4-field accounting (input/output/cache_read/cache_write).

## Members
- `lib.ts`: PresetRange, TimeRange (含 custom), DateRange, ScanWindow, formatTokens, localDateString, dateRange, clampToWindow, recordTotal
- `hooks/useUsage.ts`: single truth source, effectiveCustom memo (auto-clamp on refresh), setTimeRange(PresetRange), switchToCustom (首次继承/之后恢复)
- `pages/UsagePage.tsx`: main dashboard (summary cards, daily chart, model table with breakdown)
- `components/TimeRangeTab.tsx`: Today/7D/30D/Custom pill selector + date picker (扫描窗口边界, onChange(PresetRange), loading 时 disabled)
- `components/SummaryCards.tsx`: 4-card grid (Total, Sent, Received, Cache Hit)
- `components/DailyChart.tsx`: CSS horizontal bar chart (Claude=text/80, Codex=text/30)
- `components/ModelTable.tsx`: model distribution table with input/output/cache columns

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
