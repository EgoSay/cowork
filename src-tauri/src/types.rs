/**
 * [INPUT]: 依赖 serde 的序列化能力
 * [OUTPUT]: 对外提供 Tool, SkillFormat, Status 枚举
 * [POS]: 全局共享类型，被 features/ 和 config 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    ClaudeCode,
    Codex,
    Cursor,
    Trae,
}

impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tool::ClaudeCode => write!(f, "Claude Code"),
            Tool::Codex => write!(f, "Codex"),
            Tool::Cursor => write!(f, "Cursor"),
            Tool::Trae => write!(f, "Trae"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SkillFormat {
    SkillMd,
    AgentsMd,
    CursorMdc,
    TraeRules,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Active,
    Disabled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_display() {
        assert_eq!(Tool::ClaudeCode.to_string(), "Claude Code");
        assert_eq!(Tool::Codex.to_string(), "Codex");
        assert_eq!(Tool::Cursor.to_string(), "Cursor");
        assert_eq!(Tool::Trae.to_string(), "Trae");
    }

    #[test]
    fn tool_serialization_roundtrip() {
        let tool = Tool::ClaudeCode;
        let json = serde_json::to_string(&tool).unwrap();
        assert_eq!(json, "\"claude_code\"");
        let parsed: Tool = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, tool);
    }

    #[test]
    fn status_serialization() {
        let active = Status::Active;
        let json = serde_json::to_string(&active).unwrap();
        assert_eq!(json, "\"active\"");
    }
}
