# features/projects/
> L2 | Parent: src-tauri/src/features/

Project session 扫描、标注、缓存。从 ~/.claude/projects/ 提取会话元数据。

## Members
- `mod.rs`: 模块声明入口
- `types.rs`: ProjectMeta, SessionMeta, SessionAnnotation, ProjectData, CacheEntry, ProjectsCache 数据结构
- `scanner.rs`: parse_session_meta() JSONL 会话解析器，提取标题/计数/时间戳
- `annotations.rs`: 会话标注 CRUD（待实现）
- `commands.rs`: Tauri IPC 命令（待实现）

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
