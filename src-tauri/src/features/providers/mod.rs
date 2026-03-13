/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 types, store, writer, commands 子模块
 * [POS]: providers 功能入口，API 供应商管理
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
pub mod types;
pub mod store;
pub mod writer;
pub mod commands;
