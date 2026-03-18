/**
 * [INPUT]: 依赖 crate::types 的 Tool, SkillFormat, Status 枚举
 * [OUTPUT]: 对外提供 SkillMeta, SkillDetail, PushTarget, PushResult, EnableResult, MigrateReport, SyncReport, VerifyReport
 * [POS]: skills 功能的数据类型，被 scanner/commands/pusher/hub 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use crate::types::{SkillFormat, Status, Tool};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_tool: Tool,
    pub file_path: String,
    pub format: SkillFormat,
    pub status: Status,
    pub version: Option<String>,
    pub modified_at: Option<i64>,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetail {
    pub meta: SkillMeta,
    pub content: String,
    pub push_status: Vec<PushTarget>,
    pub dir_name: Option<String>,  // skillshub 目录名，用于 enable/disable/delete
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushTarget {
    pub tool: Tool,
    pub deployed: bool,
    pub target_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushResult {
    Success { path: String },
    AlreadyExists { path: String },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnableResult {
    Success { path: String },
    AlreadyEnabled { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrateReport {
    pub copied: Vec<String>,
    pub symlinks_updated: Vec<(Tool, String)>,
    pub errors: Vec<String>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    pub imported: Vec<(Tool, String)>,
    pub skipped: Vec<(Tool, String, String)>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    pub ok: Vec<(Tool, String)>,
    pub broken: Vec<(Tool, String, String)>,
}
