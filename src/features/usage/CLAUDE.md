# features/usage/
> L2 | Parent: src/features/

Token usage monitoring dashboard. Unified 4-field accounting (input/output/cache_read/cache_write).

## Members
- `lib.ts`: PresetRange, TimeRange (含 custom), DateRange, ScanWindow, formatTokens, localDateString, dateRange, clampToWindow, recordTotal
- `hooks/useUsage.ts`: single truth source, displayFrom/displayTo 始终反映当前筛选范围, effectiveCustom memo (auto-clamp on refresh)
- `pages/UsagePage.tsx`: main dashboard (summary cards, daily chart, model table with breakdown)
- `components/DatePicker.tsx`: 暗色日历下拉（月份导航, min/max 约束, click-outside 关闭, left/right 对齐）
- `components/TimeRangeTab.tsx`: Today/7D/30D preset + 始终可见的 DatePicker 范围选择器（修改即切换 custom）
- `components/SummaryCards.tsx`: 4-card grid (Total, Sent, Received, Cache Hit)
- `components/DailyChart.tsx`: CSS horizontal bar chart (Claude=text/80, Codex=text/30)
- `components/ModelTable.tsx`: model distribution table with input/output/cache columns

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
