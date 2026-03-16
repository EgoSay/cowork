/**
 * [INPUT]: 依赖 sha2::Sha256, dirs::home_dir 的哈希和路径能力
 * [OUTPUT]: 对外提供 expand_tilde, hash_content, path_to_id, file_modified_at, cowork_dir, file_mtime_secs
 * [POS]: shared/fs_utils 的核心工具集，被 scanner 和 config 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// 展开 ~ 到用户主目录
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

/// 计算内容的 SHA-256 哈希
pub fn hash_content(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

/// 计算文件路径的稳定 ID（截取前 16 位）
pub fn path_to_id(path: &Path) -> String {
    hash_content(path.to_string_lossy().as_bytes())[..16].to_string()
}

/// 获取文件修改时间
pub fn file_modified_at(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// 返回 ~/.cowork/ 目录路径
pub fn cowork_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cowork")
}

/// 获取文件 mtime（Unix 秒精度）
pub fn file_mtime_secs(path: &Path) -> Option<i64> {
    file_modified_at(path)?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_replaces_home() {
        let result = expand_tilde("~/test/path");
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, home.join("test/path"));
    }

    #[test]
    fn expand_tilde_absolute_path_unchanged() {
        let result = expand_tilde("/usr/local/bin");
        assert_eq!(result, PathBuf::from("/usr/local/bin"));
    }

    #[test]
    fn expand_tilde_relative_path_unchanged() {
        let result = expand_tilde("relative/path");
        assert_eq!(result, PathBuf::from("relative/path"));
    }

    #[test]
    fn hash_content_deterministic() {
        let a = hash_content(b"hello world");
        let b = hash_content(b"hello world");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64); // SHA-256 = 64 hex chars
    }

    #[test]
    fn hash_content_different_for_different_input() {
        let a = hash_content(b"hello");
        let b = hash_content(b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn path_to_id_stable_and_16_chars() {
        let id = path_to_id(Path::new("/some/path/SKILL.md"));
        assert_eq!(id.len(), 16);
        // 同路径产生同 ID
        let id2 = path_to_id(Path::new("/some/path/SKILL.md"));
        assert_eq!(id, id2);
    }

    #[test]
    fn path_to_id_different_for_different_paths() {
        let a = path_to_id(Path::new("/path/a"));
        let b = path_to_id(Path::new("/path/b"));
        assert_ne!(a, b);
    }

    #[test]
    fn file_modified_at_nonexistent_returns_none() {
        let result = file_modified_at(Path::new("/nonexistent/file/xyz"));
        assert!(result.is_none());
    }
}
