# providers/
> L2 | 父级: /CLAUDE.md

mod.rs: 模块声明
types.rs: ProviderType, ProviderProfile, ProvidersConfig 数据结构
store.rs: TOML 持久化，load/save/default，增删改查方法
writer.rs: 写入工具原生配置（Claude Code settings.json env 段）
commands.rs: Tauri IPC 命令: get_providers, switch_provider, add/update/remove_provider

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
