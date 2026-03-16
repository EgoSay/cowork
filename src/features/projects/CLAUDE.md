# features/projects/
> L2 | Parent: src/features/

Project session history & annotation engine. Scans Claude Code JSONL sessions, groups by project, supports tagging & notes.

## Members
- `lib.ts`: TAG_OPTIONS, TagId, relativeTime, formatTime, formatDate, localDateString, yesterdayString, computeDistribution, DistributionItem
- `hooks/useProjects.ts`: single truth source, useReducer state, re-entry backgroundRefresh, flash card detection, useMemo derived (filteredProjects, selectedSessions, filteredSessions, morningFocus, tagDistribution)
- `components/ProjectCard.tsx`: 项目列表卡片，显示名称/会话数/最后活跃
- `components/SessionCard.tsx`: 会话卡片，内联标签切换按钮
- `components/TagFilter.tsx`: 标签筛选栏（全部/高效/踩坑/模板/未标注）
- `components/TimeDistribution.tsx`: 水平比例条 + 图例
- `components/MorningFocus.tsx`: 昨日回顾面板，4 统计卡 + 时间分布
- `components/TokenHeatmap.tsx`: 28 天 token 消耗热力图，自取 UsageData
- `components/FlashCard.tsx`: 新会话标注弹窗 modal

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
