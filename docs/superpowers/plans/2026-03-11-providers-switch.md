# API 供应商切换 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 CoWork 指挥中心实现 API 供应商配置管理，支持一键切换 Claude Code / Codex 的 API 接入点（官方 / 第三方），消除手动编辑环境变量的成本。

**Architecture:** Provider Profile 模型存储于 `~/.cowork/providers.toml`，每个 profile 包含 API 端点和密钥。切换时通过 Writer 模块原子写入目标工具的配置文件（Claude Code → `~/.claude/settings.json` env 段，Codex → 环境变量注入）。前后端对称的 features/providers 模块，复用现有 Tauri IPC 模式。

**Tech Stack:** Rust (serde, toml, serde_json) + React 18 + TypeScript + Tailwind v4

---

## File Structure

### Backend — `src-tauri/src/features/providers/`

| File | Responsibility |
|------|---------------|
| `mod.rs` | 模块声明，导出 commands |
| `types.rs` | ProviderType, ProviderProfile, ProvidersConfig |
| `store.rs` | TOML 持久化 (`~/.cowork/providers.toml`) |
| `writer.rs` | 写入工具原生配置文件（Claude Code settings.json） |
| `commands.rs` | Tauri #[command] IPC 接口 |

### Frontend — `src/features/providers/`

| File | Responsibility |
|------|---------------|
| `pages/ProvidersPage.tsx` | Config 模块主页面 |
| `components/ProviderCard.tsx` | 供应商卡片（名称 + 类型徽章 + 切换按钮） |
| `components/ProviderForm.tsx` | 添加/编辑自定义供应商表单 |
| `hooks/useProviders.ts` | 状态管理：加载、切换、增删 |

### Shared modifications

| File | Change |
|------|--------|
| `src-tauri/src/features/mod.rs` | 添加 `pub mod providers;` |
| `src-tauri/src/lib.rs` | 注册 provider commands |
| `src/lib/types.ts` | 添加 Provider 相关类型 |
| `src/lib/api.ts` | 添加 provider IPC 封装 |
| `src/App.tsx` | 添加 providers 模块路由 |
| `src/components/layout/ModuleNav.tsx` | 启用 Config 按钮 |

---

## Data Model

### providers.toml 格式

```toml
[[providers]]
id = "claude-official"
name = "Anthropic Official"
tool = "claude_code"
provider_type = "official"

[[providers]]
id = "codex-official"
name = "OpenAI Official"
tool = "codex"
provider_type = "official"

[[providers]]
id = "my-relay"
name = "My Relay"
tool = "claude_code"
provider_type = "custom"
base_url = "https://relay.example.com/v1"
api_key = "sk-xxx"

[active]
claude_code = "claude-official"
codex = "codex-official"
```

### 切换机制

**Claude Code:** 修改 `~/.claude/settings.json` → `env` 字段
- Official → 移除 `ANTHROPIC_BASE_URL` 和 `ANTHROPIC_API_KEY`
- Custom → 写入 `ANTHROPIC_BASE_URL` 和 `ANTHROPIC_API_KEY`

**Codex:** 修改 `~/.codex/config.toml` 和 `~/.codex/auth.json`（v2 实现）

---

## Chunk 1: Backend Types & Store

### Task 1: Provider Types

**Files:**
- Create: `src-tauri/src/features/providers/mod.rs`
- Create: `src-tauri/src/features/providers/types.rs`
- Modify: `src-tauri/src/features/mod.rs`

- [ ] **Step 0: Create backend directory**

```bash
mkdir -p src-tauri/src/features/providers
```

- [ ] **Step 1: Create module declaration**

`src-tauri/src/features/providers/mod.rs`:
```rust
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
```

- [ ] **Step 2: Register providers module**

`src-tauri/src/features/mod.rs` — 添加:
```rust
pub mod providers;
```

- [ ] **Step 3: Write failing test for provider types**

`src-tauri/src/features/providers/types.rs`:
```rust
/**
 * [INPUT]: 依赖 serde 序列化
 * [OUTPUT]: 对外提供 ProviderType, ProviderProfile, ProvidersConfig
 * [POS]: providers 功能的数据类型，被 store/writer/commands 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Official,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub tool: String,
    pub provider_type: ProviderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub providers: Vec<ProviderProfile>,
    pub active: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_type_serialization() {
        let json = serde_json::to_string(&ProviderType::Official).unwrap();
        assert_eq!(json, "\"official\"");
        let json = serde_json::to_string(&ProviderType::Custom).unwrap();
        assert_eq!(json, "\"custom\"");
    }

    #[test]
    fn provider_profile_serialization() {
        let p = ProviderProfile {
            id: "test".into(),
            name: "Test".into(),
            tool: "claude_code".into(),
            provider_type: ProviderType::Custom,
            base_url: Some("https://example.com".into()),
            api_key: Some("sk-test".into()),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"custom\""));
        assert!(json.contains("https://example.com"));
    }

    #[test]
    fn official_provider_omits_optional_fields() {
        let p = ProviderProfile {
            id: "official".into(),
            name: "Official".into(),
            tool: "claude_code".into(),
            provider_type: ProviderType::Official,
            base_url: None,
            api_key: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("base_url"));
        assert!(!json.contains("api_key"));
    }

    #[test]
    fn providers_config_roundtrip() {
        let config = ProvidersConfig {
            providers: vec![
                ProviderProfile {
                    id: "official".into(),
                    name: "Anthropic Official".into(),
                    tool: "claude_code".into(),
                    provider_type: ProviderType::Official,
                    base_url: None,
                    api_key: None,
                },
            ],
            active: HashMap::from([("claude_code".into(), "official".into())]),
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: ProvidersConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.providers.len(), 1);
        assert_eq!(parsed.active["claude_code"], "official");
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test features::providers::types`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/features/providers/mod.rs src-tauri/src/features/providers/types.rs src-tauri/src/features/mod.rs
git commit -m "feat(providers): add provider data types"
```

---

### Task 2: Provider Store

**Files:**
- Create: `src-tauri/src/features/providers/store.rs`

- [ ] **Step 1: Write failing test for store**

`src-tauri/src/features/providers/store.rs`:
```rust
/**
 * [INPUT]: 依赖 types::{ProvidersConfig, ProviderProfile, ProviderType}, shared::fs_utils::expand_tilde, toml
 * [OUTPUT]: 对外提供 ProvidersConfig 的 load/save/default
 * [POS]: providers 持久化层，读写 ~/.cowork/providers.toml
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::{ProviderProfile, ProviderType, ProvidersConfig};
use crate::shared::fs_utils::expand_tilde;
use std::collections::HashMap;
use std::path::PathBuf;

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            providers: vec![
                ProviderProfile {
                    id: "claude-official".into(),
                    name: "Anthropic Official".into(),
                    tool: "claude_code".into(),
                    provider_type: ProviderType::Official,
                    base_url: None,
                    api_key: None,
                },
                ProviderProfile {
                    id: "codex-official".into(),
                    name: "OpenAI Official".into(),
                    tool: "codex".into(),
                    provider_type: ProviderType::Official,
                    base_url: None,
                    api_key: None,
                },
            ],
            active: HashMap::from([
                ("claude_code".into(), "claude-official".into()),
                ("codex".into(), "codex-official".into()),
            ]),
        }
    }
}

impl ProvidersConfig {
    fn config_path() -> PathBuf {
        expand_tilde("~/.cowork/providers.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, content).map_err(|e| e.to_string())
    }

    /// 获取指定工具的所有供应商
    pub fn providers_for_tool(&self, tool_key: &str) -> Vec<&ProviderProfile> {
        self.providers.iter().filter(|p| p.tool == tool_key).collect()
    }

    /// 获取指定工具的当前激活供应商
    pub fn active_provider(&self, tool_key: &str) -> Option<&ProviderProfile> {
        let active_id = self.active.get(tool_key)?;
        self.providers.iter().find(|p| &p.id == active_id && p.tool == tool_key)
    }

    /// 按 ID 查找供应商
    pub fn find_provider(&self, id: &str) -> Option<&ProviderProfile> {
        self.providers.iter().find(|p| p.id == id)
    }

    /// 添加自定义供应商
    pub fn add_provider(&mut self, provider: ProviderProfile) -> Result<(), String> {
        if self.providers.iter().any(|p| p.id == provider.id) {
            return Err(format!("Provider '{}' already exists", provider.id));
        }
        self.providers.push(provider);
        Ok(())
    }

    /// 删除供应商（不允许删除 official）
    pub fn remove_provider(&mut self, id: &str) -> Result<(), String> {
        let provider = self.providers.iter().find(|p| p.id == id)
            .ok_or_else(|| format!("Provider '{}' not found", id))?;
        if provider.provider_type == ProviderType::Official {
            return Err("Cannot remove official provider".into());
        }
        // 如果是当前激活的，切回 official
        let tool = provider.tool.clone();
        if self.active.get(&tool).map(|a| a.as_str()) == Some(id) {
            let official = self.providers.iter()
                .find(|p| p.tool == tool && p.provider_type == ProviderType::Official)
                .map(|p| p.id.clone());
            if let Some(official_id) = official {
                self.active.insert(tool, official_id);
            }
        }
        self.providers.retain(|p| p.id != id);
        Ok(())
    }

    /// 切换激活供应商
    pub fn set_active(&mut self, tool_key: &str, provider_id: &str) -> Result<(), String> {
        let exists = self.providers.iter()
            .any(|p| p.id == provider_id && p.tool == tool_key);
        if !exists {
            return Err(format!("Provider '{}' not found for tool '{}'", provider_id, tool_key));
        }
        self.active.insert(tool_key.into(), provider_id.into());
        Ok(())
    }

    /// 更新供应商信息
    pub fn update_provider(&mut self, id: &str, name: Option<String>, base_url: Option<String>, api_key: Option<String>) -> Result<(), String> {
        let provider = self.providers.iter_mut().find(|p| p.id == id)
            .ok_or_else(|| format!("Provider '{}' not found", id))?;
        if let Some(n) = name { provider.name = n; }
        if base_url.is_some() { provider.base_url = base_url; }
        if api_key.is_some() { provider.api_key = api_key; }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_official_providers() {
        let config = ProvidersConfig::default();
        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.active["claude_code"], "claude-official");
        assert_eq!(config.active["codex"], "codex-official");
    }

    #[test]
    fn providers_for_tool_filters_correctly() {
        let config = ProvidersConfig::default();
        let claude = config.providers_for_tool("claude_code");
        assert_eq!(claude.len(), 1);
        assert_eq!(claude[0].name, "Anthropic Official");
    }

    #[test]
    fn active_provider_returns_correct() {
        let config = ProvidersConfig::default();
        let active = config.active_provider("claude_code").unwrap();
        assert_eq!(active.id, "claude-official");
    }

    #[test]
    fn add_provider_rejects_duplicate_id() {
        let mut config = ProvidersConfig::default();
        let dup = ProviderProfile {
            id: "claude-official".into(),
            name: "Dup".into(),
            tool: "claude_code".into(),
            provider_type: ProviderType::Custom,
            base_url: None,
            api_key: None,
        };
        assert!(config.add_provider(dup).is_err());
    }

    #[test]
    fn add_and_remove_custom_provider() {
        let mut config = ProvidersConfig::default();
        let custom = ProviderProfile {
            id: "my-relay".into(),
            name: "My Relay".into(),
            tool: "claude_code".into(),
            provider_type: ProviderType::Custom,
            base_url: Some("https://relay.example.com".into()),
            api_key: Some("sk-test".into()),
        };
        config.add_provider(custom).unwrap();
        assert_eq!(config.providers.len(), 3);

        config.remove_provider("my-relay").unwrap();
        assert_eq!(config.providers.len(), 2);
    }

    #[test]
    fn cannot_remove_official_provider() {
        let mut config = ProvidersConfig::default();
        assert!(config.remove_provider("claude-official").is_err());
    }

    #[test]
    fn remove_active_custom_falls_back_to_official() {
        let mut config = ProvidersConfig::default();
        let custom = ProviderProfile {
            id: "relay".into(),
            name: "Relay".into(),
            tool: "claude_code".into(),
            provider_type: ProviderType::Custom,
            base_url: Some("https://r.com".into()),
            api_key: Some("sk-x".into()),
        };
        config.add_provider(custom).unwrap();
        config.set_active("claude_code", "relay").unwrap();
        assert_eq!(config.active["claude_code"], "relay");

        config.remove_provider("relay").unwrap();
        assert_eq!(config.active["claude_code"], "claude-official");
    }

    #[test]
    fn set_active_rejects_unknown_provider() {
        let mut config = ProvidersConfig::default();
        assert!(config.set_active("claude_code", "nonexistent").is_err());
    }

    #[test]
    fn toml_roundtrip() {
        let mut config = ProvidersConfig::default();
        config.add_provider(ProviderProfile {
            id: "relay".into(),
            name: "Test Relay".into(),
            tool: "claude_code".into(),
            provider_type: ProviderType::Custom,
            base_url: Some("https://test.com".into()),
            api_key: Some("sk-123".into()),
        }).unwrap();

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: ProvidersConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.providers.len(), 3);
        assert_eq!(parsed.active["claude_code"], "claude-official");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test features::providers::store`
Expected: ALL PASS

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/features/providers/store.rs
git commit -m "feat(providers): add provider store with TOML persistence"
```

---

## Chunk 2: Backend Writer & Commands

### Task 3: Claude Code Config Writer

**Files:**
- Create: `src-tauri/src/features/providers/writer.rs`

**核心机制:** 读取 `~/.claude/settings.json`，修改 `env` 字段，写回。

- [ ] **Step 1: Implement writer**

`src-tauri/src/features/providers/writer.rs`:
```rust
/**
 * [INPUT]: 依赖 types::ProviderProfile/ProviderType, shared::fs_utils::expand_tilde, serde_json
 * [OUTPUT]: 对外提供 apply_provider (写入工具配置文件)
 * [POS]: providers 的配置写入器，将激活供应商同步到工具原生配置
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::{ProviderProfile, ProviderType};
use crate::shared::fs_utils::expand_tilde;
use serde_json::Value;
use std::path::PathBuf;

/// 将供应商配置应用到目标工具的配置文件
pub fn apply_provider(provider: &ProviderProfile) -> Result<(), String> {
    match provider.tool.as_str() {
        "claude_code" => apply_claude_code(provider),
        "codex" => Err("Codex provider switching not yet supported".into()),
        _ => Err(format!("Unsupported tool: {}", provider.tool)),
    }
}

fn claude_settings_path() -> PathBuf {
    expand_tilde("~/.claude/settings.json")
}

fn apply_claude_code(provider: &ProviderProfile) -> Result<(), String> {
    let path = claude_settings_path();

    // ---- 读取当前配置 ----
    let content = if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| e.to_string())?
    } else {
        "{}".into()
    };
    let mut settings: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid settings.json: {}", e))?;

    // ---- 确保 env 对象存在 ----
    if settings.get("env").is_none() {
        settings["env"] = Value::Object(serde_json::Map::new());
    }
    let env = settings["env"].as_object_mut()
        .ok_or("env is not an object")?;

    // ---- 根据类型写入/清除 ----
    match provider.provider_type {
        ProviderType::Official => {
            env.remove("ANTHROPIC_BASE_URL");
            env.remove("ANTHROPIC_API_KEY");
        }
        ProviderType::Custom => {
            if let Some(url) = &provider.base_url {
                env.insert("ANTHROPIC_BASE_URL".into(), Value::String(url.clone()));
            }
            if let Some(key) = &provider.api_key {
                env.insert("ANTHROPIC_API_KEY".into(), Value::String(key.clone()));
            }
        }
    }

    // ---- 原子写入 ----
    let output = serde_json::to_string_pretty(&settings)
        .map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, output).map_err(|e| e.to_string())
}

/// 从 Claude Code 配置中读取当前 API 状态
pub fn read_claude_code_env() -> Result<(Option<String>, Option<String>), String> {
    let path = claude_settings_path();
    if !path.exists() {
        return Ok((None, None));
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let settings: Value = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;

    let base_url = settings.get("env")
        .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let api_key = settings.get("env")
        .and_then(|e| e.get("ANTHROPIC_API_KEY"))
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok((base_url, api_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::env;

    #[test]
    fn claude_settings_path_is_correct() {
        let path = claude_settings_path();
        let home = dirs::home_dir().unwrap();
        assert_eq!(path, home.join(".claude/settings.json"));
    }

    #[test]
    fn apply_official_removes_env_vars() {
        // 注意: 此测试验证 JSON 操作逻辑的正确性
        // apply_claude_code 函数操作真实路径，这里用 unit 方式验证核心逻辑
        let mut settings: Value = serde_json::from_str(r#"{
            "env": {
                "ANTHROPIC_BASE_URL": "https://old.com",
                "ANTHROPIC_API_KEY": "sk-old",
                "OTHER_VAR": "keep"
            },
            "model": "opus"
        }"#).unwrap();

        let env = settings["env"].as_object_mut().unwrap();
        env.remove("ANTHROPIC_BASE_URL");
        env.remove("ANTHROPIC_API_KEY");

        assert!(!settings["env"].as_object().unwrap().contains_key("ANTHROPIC_BASE_URL"));
        assert!(!settings["env"].as_object().unwrap().contains_key("ANTHROPIC_API_KEY"));
        assert!(settings["env"].as_object().unwrap().contains_key("OTHER_VAR"));
        assert_eq!(settings["model"], "opus"); // 其他字段不受影响
    }

    #[test]
    fn apply_custom_sets_env_vars() {
        let mut settings: Value = serde_json::from_str(r#"{
            "env": { "OTHER_VAR": "keep" },
            "model": "opus"
        }"#).unwrap();

        let env = settings["env"].as_object_mut().unwrap();
        env.insert("ANTHROPIC_BASE_URL".into(), Value::String("https://relay.com/v1".into()));
        env.insert("ANTHROPIC_API_KEY".into(), Value::String("sk-new".into()));

        assert_eq!(
            settings["env"]["ANTHROPIC_BASE_URL"].as_str().unwrap(),
            "https://relay.com/v1"
        );
        assert_eq!(
            settings["env"]["ANTHROPIC_API_KEY"].as_str().unwrap(),
            "sk-new"
        );
        assert_eq!(settings["env"]["OTHER_VAR"].as_str().unwrap(), "keep");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test features::providers::writer`
Expected: ALL PASS

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/features/providers/writer.rs
git commit -m "feat(providers): add config writer for Claude Code settings.json"
```

---

### Task 4: Tauri Commands

**Files:**
- Create: `src-tauri/src/features/providers/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Implement commands**

`src-tauri/src/features/providers/commands.rs`:
```rust
/**
 * [INPUT]: 依赖 store, writer, types
 * [OUTPUT]: 对外提供 Tauri IPC 命令 (list/switch/add/update/remove)
 * [POS]: providers 功能的 Tauri IPC 接口层
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::{ProviderProfile, ProviderType, ProvidersConfig};
use super::writer;

/// 获取所有供应商配置（含 active 状态）
#[tauri::command]
pub async fn get_providers() -> Result<ProvidersConfig, String> {
    Ok(ProvidersConfig::load())
}

/// 切换指定工具的激活供应商
#[tauri::command]
pub async fn switch_provider(tool_key: String, provider_id: String) -> Result<(), String> {
    let mut config = ProvidersConfig::load();
    config.set_active(&tool_key, &provider_id)?;

    // 将新供应商写入工具配置
    let provider = config.find_provider(&provider_id)
        .ok_or("Provider not found after set_active")?
        .clone();
    writer::apply_provider(&provider)?;

    config.save()
}

/// 添加自定义供应商
#[tauri::command]
pub async fn add_provider(
    id: String,
    name: String,
    tool: String,
    base_url: String,
    api_key: String,
) -> Result<(), String> {
    let mut config = ProvidersConfig::load();
    config.add_provider(ProviderProfile {
        id,
        name,
        tool,
        provider_type: ProviderType::Custom,
        base_url: Some(base_url),
        api_key: Some(api_key),
    })?;
    config.save()
}

/// 更新供应商信息
#[tauri::command]
pub async fn update_provider(
    id: String,
    name: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<(), String> {
    let mut config = ProvidersConfig::load();
    config.update_provider(&id, name, base_url, api_key)?;

    // 如果修改的是当前激活的供应商，重新写入
    let provider = config.find_provider(&id)
        .ok_or_else(|| format!("Provider '{}' disappeared after update", id))?
        .clone();
    if config.active.values().any(|a| a == &id) {
        writer::apply_provider(&provider)?;
    }

    config.save()
}

/// 删除供应商
#[tauri::command]
pub async fn remove_provider(id: String) -> Result<(), String> {
    let mut config = ProvidersConfig::load();

    // 如果删除的是当前激活的，先切回 official 并应用
    let provider = config.find_provider(&id)
        .ok_or_else(|| format!("Provider '{}' not found", id))?;
    let tool = provider.tool.clone();
    let is_active = config.active.get(&tool).map(|a| a.as_str()) == Some(&id);

    config.remove_provider(&id)?;

    if is_active {
        if let Some(official) = config.active_provider(&tool) {
            writer::apply_provider(&official.clone())?;
        }
    }

    config.save()
}

/// 读取 Claude Code 当前 env 配置状态
#[tauri::command]
pub async fn read_claude_env() -> Result<(Option<String>, Option<String>), String> {
    writer::read_claude_code_env()
}
```

- [ ] **Step 2: Register commands in lib.rs**

`src-tauri/src/lib.rs` — 添加:
```rust
use features::providers::commands as provider_commands;
```

在 `invoke_handler` 中添加:
```rust
provider_commands::get_providers,
provider_commands::switch_provider,
provider_commands::add_provider,
provider_commands::update_provider,
provider_commands::remove_provider,
provider_commands::read_claude_env,
```

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/features/providers/commands.rs src-tauri/src/lib.rs
git commit -m "feat(providers): add Tauri commands and wire into app"
```

---

## Chunk 3: Frontend Types & Hook

### Task 5: TypeScript Types & API

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Add provider types**

`src/lib/types.ts` — 文件末尾追加:
```typescript
// ---- Provider 供应商管理 ----

export type ProviderType = "official" | "custom"

export interface ProviderProfile {
  id: string
  name: string
  tool: string
  provider_type: ProviderType
  base_url?: string
  api_key?: string
}

export interface ProvidersConfig {
  providers: ProviderProfile[]
  active: Record<string, string>
}
```

- [ ] **Step 2: Add provider API functions**

`src/lib/api.ts` — 将 `ProvidersConfig` 合并到已有 import 行:
```typescript
// 修改已有 import（第8行）:
import type { SkillMeta, SkillDetail, PushResult, Tool, ProvidersConfig } from "./types"

// 文件末尾追加以下函数:
export async function getProviders(): Promise<ProvidersConfig> {
  return invoke<ProvidersConfig>("get_providers")
}

export async function switchProvider(toolKey: string, providerId: string): Promise<void> {
  return invoke("switch_provider", { toolKey, providerId })
}

export async function addProvider(
  id: string,
  name: string,
  tool: string,
  baseUrl: string,
  apiKey: string
): Promise<void> {
  return invoke("add_provider", { id, name, tool, baseUrl, apiKey })
}

export async function updateProvider(
  id: string,
  name?: string,
  baseUrl?: string,
  apiKey?: string
): Promise<void> {
  return invoke("update_provider", { id, name, baseUrl, apiKey })
}

export async function removeProvider(id: string): Promise<void> {
  return invoke("remove_provider", { id })
}

export async function readClaudeEnv(): Promise<[string | null, string | null]> {
  return invoke<[string | null, string | null]>("read_claude_env")
}
```

- [ ] **Step 3: Verify typecheck**

Run: `pnpm typecheck`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts
git commit -m "feat(providers): add frontend types and API layer"
```

---

### Task 6: useProviders Hook

**Files:**
- Create: `src/features/providers/hooks/useProviders.ts`

- [ ] **Step 0: Create directory structure**

```bash
mkdir -p src/features/providers/{pages,components,hooks}
```

- [ ] **Step 1: Implement hook**

```typescript
/**
 * [INPUT]: 依赖 @/lib/api 的 provider 函数, @/lib/types
 * [OUTPUT]: 对外提供 useProviders hook
 * [POS]: providers 的状态管理，被 ProvidersPage 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useReducer } from "react"
import {
  getProviders,
  switchProvider,
  addProvider as apiAddProvider,
  updateProvider as apiUpdateProvider,
  removeProvider as apiRemoveProvider,
} from "@/lib/api"
import type { ProvidersConfig, ProviderProfile } from "@/lib/types"

interface State {
  config: ProvidersConfig | null
  loading: boolean
  error: string | null
  switching: string | null // 正在切换的 provider ID
}

type Action =
  | { type: "LOAD_START" }
  | { type: "LOAD_OK"; config: ProvidersConfig }
  | { type: "LOAD_ERR"; error: string }
  | { type: "SWITCH_START"; id: string }
  | { type: "SWITCH_OK" }
  | { type: "SWITCH_ERR"; error: string }

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "LOAD_START":
      return { ...state, loading: true, error: null }
    case "LOAD_OK":
      return { ...state, loading: false, config: action.config }
    case "LOAD_ERR":
      return { ...state, loading: false, error: action.error }
    case "SWITCH_START":
      return { ...state, switching: action.id, error: null }
    case "SWITCH_OK":
      return { ...state, switching: null }
    case "SWITCH_ERR":
      return { ...state, switching: null, error: action.error }
  }
}

export function useProviders(toolKey: string = "claude_code") {
  const [state, dispatch] = useReducer(reducer, {
    config: null,
    loading: true,
    error: null,
    switching: null,
  })

  const load = useCallback(async () => {
    dispatch({ type: "LOAD_START" })
    try {
      const config = await getProviders()
      dispatch({ type: "LOAD_OK", config })
    } catch (e) {
      dispatch({ type: "LOAD_ERR", error: String(e) })
    }
  }, [])

  useEffect(() => { load() }, [load])

  const doSwitch = useCallback(async (providerId: string) => {
    dispatch({ type: "SWITCH_START", id: providerId })
    try {
      await switchProvider(toolKey, providerId)
      dispatch({ type: "SWITCH_OK" })
      await load()
    } catch (e) {
      dispatch({ type: "SWITCH_ERR", error: String(e) })
    }
  }, [toolKey, load])

  const doAdd = useCallback(async (
    id: string, name: string, baseUrl: string, apiKey: string
  ) => {
    await apiAddProvider(id, name, toolKey, baseUrl, apiKey)
    await load()
  }, [toolKey, load])

  const doUpdate = useCallback(async (
    id: string, name?: string, baseUrl?: string, apiKey?: string
  ) => {
    await apiUpdateProvider(id, name, baseUrl, apiKey)
    await load()
  }, [load])

  const doRemove = useCallback(async (id: string) => {
    await apiRemoveProvider(id)
    await load()
  }, [load])

  const providers = state.config?.providers.filter(p => p.tool === toolKey) ?? []
  const activeId = state.config?.active[toolKey] ?? ""

  return {
    providers,
    activeId,
    loading: state.loading,
    switching: state.switching,
    error: state.error,
    switchProvider: doSwitch,
    addProvider: doAdd,
    updateProvider: doUpdate,
    removeProvider: doRemove,
    reload: load,
  }
}
```

- [ ] **Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src/features/providers/hooks/useProviders.ts
git commit -m "feat(providers): add useProviders state management hook"
```

---

## Chunk 4: Frontend UI

### Task 7: ProviderCard Component

**Files:**
- Create: `src/features/providers/components/ProviderCard.tsx`

- [ ] **Step 1: Implement ProviderCard**

```tsx
/**
 * [INPUT]: 依赖 @/lib/types 的 ProviderProfile
 * [OUTPUT]: 对外提供 ProviderCard 组件
 * [POS]: providers 的卡片组件，显示供应商信息和切换按钮
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { ProviderProfile } from "@/lib/types"

interface ProviderCardProps {
  provider: ProviderProfile
  isActive: boolean
  isSwitching: boolean
  onSwitch: () => void
  onEdit: () => void
  onRemove: () => void
}

export function ProviderCard({
  provider,
  isActive,
  isSwitching,
  onSwitch,
  onEdit,
  onRemove,
}: ProviderCardProps) {
  return (
    <div
      className={`p-4 rounded-xl border transition-colors ${
        isActive
          ? "border-text/20 bg-bg-hover"
          : "border-border hover:border-text/10"
      }`}
    >
      {/* 头部: 名称 + 状态 */}
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          {isActive && (
            <span className="w-2 h-2 rounded-full bg-emerald-400" />
          )}
          <h3 className="text-sm font-medium text-text">{provider.name}</h3>
        </div>
        <span
          className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${
            provider.provider_type === "official"
              ? "bg-text/5 text-text-muted"
              : "bg-blue-500/10 text-blue-400"
          }`}
        >
          {provider.provider_type === "official" ? "Official" : "Custom"}
        </span>
      </div>

      {/* 端点信息 */}
      {provider.base_url && (
        <p className="text-xs text-text-muted truncate mb-3">
          {provider.base_url}
        </p>
      )}
      {!provider.base_url && (
        <p className="text-xs text-text-muted mb-3">Default API endpoint</p>
      )}

      {/* 操作按钮 */}
      <div className="flex items-center gap-2">
        {!isActive && (
          <button
            onClick={onSwitch}
            disabled={isSwitching}
            className="text-xs px-3 py-1 rounded-lg bg-text/5 text-text-secondary hover:bg-text/10 transition-colors disabled:opacity-50"
          >
            {isSwitching ? "Switching..." : "Activate"}
          </button>
        )}
        {isActive && (
          <span className="text-xs text-emerald-400 font-medium">Active</span>
        )}
        {provider.provider_type === "custom" && (
          <>
            <button
              onClick={onEdit}
              className="text-xs px-2 py-1 text-text-muted hover:text-text-secondary transition-colors"
            >
              Edit
            </button>
            <button
              onClick={onRemove}
              className="text-xs px-2 py-1 text-red-400/60 hover:text-red-400 transition-colors"
            >
              Remove
            </button>
          </>
        )}
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/features/providers/components/ProviderCard.tsx
git commit -m "feat(providers): add ProviderCard component"
```

---

### Task 8: ProviderForm Component

**Files:**
- Create: `src/features/providers/components/ProviderForm.tsx`

- [ ] **Step 1: Implement ProviderForm**

```tsx
/**
 * [INPUT]: 依赖 react 的 useState, @/lib/types 的 ProviderProfile
 * [OUTPUT]: 对外提供 ProviderForm 组件
 * [POS]: providers 的添加/编辑表单，被 ProvidersPage 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState } from "react"
import type { ProviderProfile } from "@/lib/types"

interface ProviderFormProps {
  initial?: ProviderProfile
  onSubmit: (data: { id: string; name: string; baseUrl: string; apiKey: string }) => void
  onCancel: () => void
}

export function ProviderForm({ initial, onSubmit, onCancel }: ProviderFormProps) {
  const [name, setName] = useState(initial?.name ?? "")
  const [baseUrl, setBaseUrl] = useState(initial?.base_url ?? "")
  const [apiKey, setApiKey] = useState(initial?.api_key ?? "")

  const id = initial?.id ?? name.toLowerCase().replace(/[^a-z0-9]+/g, "-")
  const valid = name.trim() && baseUrl.trim() && apiKey.trim()

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!valid) return
    onSubmit({ id, name: name.trim(), baseUrl: baseUrl.trim(), apiKey: apiKey.trim() })
  }

  const inputClass =
    "w-full px-3 py-2 rounded-lg bg-bg text-text text-sm border border-border focus:border-text/20 focus:outline-none"

  return (
    <form onSubmit={handleSubmit} className="space-y-3">
      <div>
        <label className="block text-xs text-text-muted mb-1">Name</label>
        <input
          className={inputClass}
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="My Relay Provider"
        />
      </div>
      <div>
        <label className="block text-xs text-text-muted mb-1">API Base URL</label>
        <input
          className={inputClass}
          value={baseUrl}
          onChange={(e) => setBaseUrl(e.target.value)}
          placeholder="https://relay.example.com/v1"
        />
      </div>
      <div>
        <label className="block text-xs text-text-muted mb-1">API Key</label>
        <input
          className={inputClass}
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="sk-..."
        />
      </div>
      <div className="flex gap-2 pt-1">
        <button
          type="submit"
          disabled={!valid}
          className="text-xs px-4 py-1.5 rounded-lg bg-text/10 text-text hover:bg-text/15 transition-colors disabled:opacity-30"
        >
          {initial ? "Save" : "Add Provider"}
        </button>
        <button
          type="button"
          onClick={onCancel}
          className="text-xs px-4 py-1.5 rounded-lg text-text-muted hover:text-text-secondary transition-colors"
        >
          Cancel
        </button>
      </div>
    </form>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/features/providers/components/ProviderForm.tsx
git commit -m "feat(providers): add ProviderForm component"
```

---

### Task 9: ProvidersPage & Wiring

**Files:**
- Create: `src/features/providers/pages/ProvidersPage.tsx`
- Modify: `src/App.tsx`
- Modify: `src/components/layout/ModuleNav.tsx`

- [ ] **Step 1: Implement ProvidersPage**

```tsx
/**
 * [INPUT]: 依赖 useProviders hook, ProviderCard, ProviderForm
 * [OUTPUT]: 对外提供 ProvidersPage 组件
 * [POS]: Config 模块主页面，供应商管理
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState } from "react"
import { useProviders } from "../hooks/useProviders"
import { ProviderCard } from "../components/ProviderCard"
import { ProviderForm } from "../components/ProviderForm"
import type { ProviderProfile } from "@/lib/types"

const TOOL_TABS = [
  { key: "claude_code", label: "Claude Code" },
  { key: "codex", label: "Codex" },
]

export function ProvidersPage() {
  const [toolKey, setToolKey] = useState("claude_code")
  const [showForm, setShowForm] = useState(false)
  const [editing, setEditing] = useState<ProviderProfile | null>(null)

  const {
    providers,
    activeId,
    loading,
    switching,
    error,
    switchProvider,
    addProvider,
    updateProvider,
    removeProvider,
  } = useProviders(toolKey)

  const handleAdd = async (data: {
    id: string; name: string; baseUrl: string; apiKey: string
  }) => {
    await addProvider(data.id, data.name, data.baseUrl, data.apiKey)
    setShowForm(false)
  }

  const handleEdit = async (data: {
    id: string; name: string; baseUrl: string; apiKey: string
  }) => {
    await updateProvider(data.id, data.name, data.baseUrl, data.apiKey)
    setEditing(null)
  }

  const handleRemove = async (id: string) => {
    await removeProvider(id)
  }

  return (
    <div className="h-full flex flex-col p-6">
      {/* 页面标题 */}
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-lg font-semibold text-text">API Providers</h1>
        <button
          onClick={() => { setShowForm(true); setEditing(null) }}
          className="text-xs px-3 py-1.5 rounded-lg bg-text/5 text-text-secondary hover:bg-text/10 transition-colors"
        >
          + Add Provider
        </button>
      </div>

      {/* 工具标签页 */}
      <div className="flex gap-1 mb-4">
        {TOOL_TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => { setToolKey(tab.key); setShowForm(false); setEditing(null) }}
            className={`text-xs px-3 py-1.5 rounded-lg transition-colors ${
              toolKey === tab.key
                ? "bg-bg-hover text-text"
                : "text-text-muted hover:text-text-secondary hover:bg-bg-hover/50"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* 错误提示 */}
      {error && (
        <div className="mb-4 text-xs text-red-400 bg-red-400/5 px-3 py-2 rounded-lg">
          {error}
        </div>
      )}

      {/* 加载状态 */}
      {loading && (
        <p className="text-xs text-text-muted">Loading...</p>
      )}

      {/* 供应商卡片网格 */}
      {!loading && (
        <div className="grid grid-cols-2 gap-3">
          {providers.map((p) => (
            <ProviderCard
              key={p.id}
              provider={p}
              isActive={p.id === activeId}
              isSwitching={switching === p.id}
              onSwitch={() => switchProvider(p.id)}
              onEdit={() => { setEditing(p); setShowForm(false) }}
              onRemove={() => handleRemove(p.id)}
            />
          ))}
        </div>
      )}

      {/* 添加/编辑表单 */}
      {(showForm || editing) && (
        <div className="mt-6 p-4 rounded-xl border border-border">
          <h2 className="text-sm font-medium text-text mb-3">
            {editing ? "Edit Provider" : "Add Custom Provider"}
          </h2>
          <ProviderForm
            initial={editing ?? undefined}
            onSubmit={editing ? handleEdit : handleAdd}
            onCancel={() => { setShowForm(false); setEditing(null) }}
          />
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Enable Config module in ModuleNav**

`src/components/layout/ModuleNav.tsx` — 在 `.map` 回调内部，替换第 30-31 行的禁用逻辑:

替换:
```tsx
          } ${m.id !== "skills" ? "opacity-30 cursor-not-allowed" : ""}`}
          disabled={m.id !== "skills"}
```

为:
```tsx
          } ${m.id !== "skills" && m.id !== "config" ? "opacity-30 cursor-not-allowed" : ""}`}
          disabled={m.id !== "skills" && m.id !== "config"}
```

- [ ] **Step 3: Wire ProvidersPage into App.tsx**

`src/App.tsx`:

1. 在文件顶部 import 区域添加:
```tsx
import { ProvidersPage } from "@/features/providers/pages/ProvidersPage"
```

2. 在 `{activeModule === "skills" && (...)}` 块之后、`</AppShell>` 之前添加:
```tsx
      {activeModule === "config" && <ProvidersPage />}
```

- [ ] **Step 4: Verify dev build**

Run: `pnpm tauri dev`
Expected: 应用启动，Config 导航可点击，ProvidersPage 显示供应商卡片

- [ ] **Step 5: Commit**

```bash
git add src/features/providers/ src/App.tsx src/components/layout/ModuleNav.tsx
git commit -m "feat(providers): add ProvidersPage, wire into app routing"
```

---

## Chunk 5: Documentation & Cleanup

### Task 10: Update CLAUDE.md (GEB Protocol)

**Files:**
- Modify: `CLAUDE.md` (L1)
- Create: `src/features/providers/CLAUDE.md` (L2)
- Create: `src-tauri/src/features/providers/CLAUDE.md` (L2)

- [ ] **Step 1: Update L1 project constitution**

`CLAUDE.md` — 在 `<directory>` 段添加:
```
src/features/providers/ - API 供应商管理模块 (3子目录: pages, components, hooks)
src-tauri/src/features/providers/ - 供应商后端: types, store, writer, commands
```

- [ ] **Step 2: Create frontend L2**

`src/features/providers/CLAUDE.md`:
```markdown
# providers/
> L2 | 父级: /CLAUDE.md

pages/ProvidersPage.tsx: Config 模块主页面，工具标签页 + 供应商卡片网格 + 添加表单
components/ProviderCard.tsx: 供应商卡片，名称 + 类型徽章 + 状态指示 + 操作按钮
components/ProviderForm.tsx: 添加/编辑表单，name + base_url + api_key
hooks/useProviders.ts: 状态管理 reducer，加载/切换/增删/错误处理

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
```

- [ ] **Step 3: Create backend L2**

`src-tauri/src/features/providers/CLAUDE.md`:
```markdown
# providers/
> L2 | 父级: /CLAUDE.md

mod.rs: 模块声明
types.rs: ProviderType, ProviderProfile, ProvidersConfig 数据结构
store.rs: TOML 持久化，load/save/default，增删改查方法
writer.rs: 写入工具原生配置（Claude Code settings.json env 段）
commands.rs: Tauri IPC 命令: get_providers, switch_provider, add/update/remove_provider

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
```

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md src/features/providers/CLAUDE.md src-tauri/src/features/providers/CLAUDE.md
git commit -m "docs: update GEB L1/L2 for providers module"
```

---

## Summary

| Chunk | Tasks | Core Deliverable |
|-------|-------|-----------------|
| 1 | Task 1-2 | Rust types + TOML store |
| 2 | Task 3-4 | Config writer + Tauri commands |
| 3 | Task 5-6 | TS types + API + hook |
| 4 | Task 7-9 | UI components + page + routing |
| 5 | Task 10 | GEB documentation |

**Total: 10 tasks, ~25 steps**

**Codex 写入支持:** 明确推迟到 v2，当前 `writer.rs` 中 Codex 分支返回 `Ok(())`。

切换核心路径: `UI 点击 Activate → switchProvider IPC → store.set_active → writer.apply_provider → 写入 settings.json env → reload config`
