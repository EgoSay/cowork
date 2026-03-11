/**
 * [INPUT]: 依赖 sha2, dirs 的哈希和路径能力
 * [OUTPUT]: 对外提供 fs_utils 模块（expand_tilde, hash_content, path_to_id, file_modified_at）
 * [POS]: shared/ 的入口，被 features/ 各模块消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
pub mod fs_utils;
