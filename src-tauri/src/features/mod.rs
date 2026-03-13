/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 skills, usage, providers 模块
 * [POS]: features/ 入口，按功能模块组织
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
pub mod skills;
pub mod usage;
pub mod providers;
