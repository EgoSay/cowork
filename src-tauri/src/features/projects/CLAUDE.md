# features/projects/
> L2 | Parent: src-tauri/src/features/

Project session 扫描、标注、缓存。从 ~/.claude/projects/ 提取会话元数据。

## Members
- `mod.rs`: 模块声明入口
- `types.rs`: ProjectMeta, SessionMeta, SessionMessage, SessionAnnotation, ProjectData, CacheEntry, ProjectsCache 数据结构
- `scanner.rs`: parse_session_meta() JSONL 解析 + parse_session_messages() 完整消息解析 + extract_project_name() 名称提取 + scan_from_dir()/scan_all() 目录扫描 + JSON 缓存加速
- `annotations.rs`: load/save/upsert/remove 会话标注 CRUD，持久化到 ~/.cowork/annotations.toml
- `commands.rs`: Tauri IPC 命令: scan_projects, get_session_messages, resume_session, annotate_session, get_annotations, remove_annotation

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
