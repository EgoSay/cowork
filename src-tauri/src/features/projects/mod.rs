/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 types, scanner, annotations, commands 子模块
 * [POS]: projects 功能模块入口
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
pub mod types;
pub mod scanner;
pub mod annotations;
pub mod commands;
