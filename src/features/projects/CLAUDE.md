# features/projects/
> L2 | Parent: src/features/

Project session history & annotation engine. Scans Claude Code JSONL sessions, groups by project, supports tagging & notes.

## Members
- `lib.ts`: TAG_OPTIONS, TagId, relativeTime, formatTime, formatDate, localDateString, yesterdayString, computeDistribution, DistributionItem
- `hooks/useProjects.ts`: single truth source, useReducer state, re-entry backgroundRefresh, flash card detection, useMemo derived (filteredProjects, selectedSessions, filteredSessions, morningFocus, tagDistribution)

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
