/**
 * [INPUT]: 依赖 types, config, shared 模块
 * [OUTPUT]: 对外提供 commands, hub, pusher, scanner, types 子模块
 * [POS]: skills 功能模块入口，被 lib.rs 注册到 Tauri
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
pub mod commands;
pub mod hub;
pub mod pusher;
pub mod scanner;
pub mod types;
