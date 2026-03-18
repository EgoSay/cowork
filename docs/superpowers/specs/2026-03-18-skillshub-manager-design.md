# SkillsHub Manager — 统一 Skill 生命周期管理

## 概述

将 skillshub 从被动的文件仓库升级为主动的中央管理器。所有 skill 生命周期操作（启用/禁用/删除/迁移/同步/校验）统一通过 hub manager 完成，以 symlink 为唯一的工具关联机制。

## 需求

1. **skillshub 路径可配置**：用户可自定义 skillshub 目录，改路径时自动复制文件 + 重建所有 symlink + 一致性校验
2. **symlink 语义的启用/禁用**：enable = 创建 symlink，disable = 删除 symlink，delete = 从 skillshub 删除 + 清理所有 symlink
3. **Sync 同步**：手动触发，扫描工具目录里的非 symlink skill → 复制到 skillshub → 替换为 symlink

## 架构决策

- **Hub Manager 取代 Pusher**：新建 `hub.rs`，废弃 `pusher.rs`。所有 skillshub 操作集中管理
- **Scanner 保持只读**：scanner 不做写操作，只负责发现和解析 skill 文件
- **symlink 是唯一关联机制**：废弃文件重命名的 enable/disable 逻辑

---

## 后端设计

### 新模块：`src-tauri/src/features/skills/hub.rs`

#### 核心函数

```rust
/// 在 tool 的 skills 目录创建指向 skillshub 的 symlink
pub fn enable(skill_dir_name: &str, tool: &Tool, config: &AppConfig) -> Result<EnableResult>

/// 删除 tool 目录里的 symlink（skill 仍在 skillshub）
pub fn disable(skill_dir_name: &str, tool: &Tool, config: &AppConfig) -> Result<()>

/// 从 skillshub 删除 skill 目录 + 清理所有工具里指向它的 symlink
pub fn delete(skill_dir_name: &str, config: &AppConfig) -> Result<()>

/// 复制 old → new，重建所有 symlink，校验一致性
pub fn migrate(old_path: &Path, new_path: &Path, config: &AppConfig) -> Result<MigrateReport>

/// 扫描各工具目录，找到非 symlink skill → 复制到 skillshub → 替换为 symlink
pub fn sync(config: &AppConfig) -> Result<SyncReport>

/// 遍历所有 symlink，检查：指向存在？目标在 skillshub 内？
pub fn verify(config: &AppConfig) -> Result<VerifyReport>
```

#### 返回类型

```rust
pub enum EnableResult {
    Success { path: String },
    AlreadyEnabled { path: String },
    Error { message: String },
}

pub struct MigrateReport {
    pub copied: Vec<String>,                    // 成功复制的 skill 名
    pub symlinks_updated: Vec<(Tool, String)>,  // (工具, skill名)
    pub errors: Vec<String>,                    // 失败项
    pub verified: bool,                         // 最终校验是否通过
}

pub struct SyncReport {
    pub imported: Vec<(Tool, String)>,  // 从哪个工具导入了什么
    pub errors: Vec<String>,
}

pub struct VerifyReport {
    pub ok: Vec<(Tool, String)>,                // 健康的 symlink
    pub broken: Vec<(Tool, String, String)>,     // (工具, skill名, 原因)
}
```

#### 关键行为

- **enable**：`symlink(skillshub/skill_name, tool_dir/skill_name)`，已存在则返回 AlreadyEnabled
- **disable**：检查 target 是 symlink 才删除，防止误删真实文件
- **delete**：先遍历所有工具目录清理 symlink，再删 skillshub 里的目录
- **migrate**：复制 old → new → 遍历所有工具目录找指向 old 的 symlink → 删除旧 symlink → 创建新 symlink → 调 `verify()` 校验
- **sync**：遍历各工具目录，`is_symlink() == false` 且含 SKILL.md 的目录 → 复制到 skillshub → 删原目录 → 创建 symlink
- **verify**：遍历所有工具目录的 symlink，检查 `read_link()` 目标是否存在且在 skillshub 内

### 废弃模块

`src-tauri/src/features/skills/pusher.rs` — 其 `push_to_tool` 逻辑并入 `hub::enable`，`skill_dir_name` 工具函数移入 `hub.rs`。

### Commands 层变更 (`commands.rs`)

#### 废弃命令

| 命令 | 原因 |
|------|------|
| `push_skill` | 被 `enable_skill` 取代 |
| 旧 `disable_skill` | 文件重命名语义，换成 symlink 语义 |
| 旧 `enable_skill` | 同上 |
| 旧 `delete_skill` | 新版走 `hub::delete` |

#### 新增/替换命令

```rust
#[tauri::command]
fn enable_skill(skill_name: String, targets: Vec<Tool>) -> Result<Vec<EnableResult>>

#[tauri::command]
fn disable_skill(skill_name: String, targets: Vec<Tool>) -> Result<()>

#[tauri::command]
fn delete_skill(skill_name: String) -> Result<()>

#[tauri::command]
fn migrate_hub(new_path: String) -> Result<MigrateReport>

#[tauri::command]
fn sync_skills() -> Result<SyncReport>

#[tauri::command]
fn verify_skills() -> Result<VerifyReport>
```

#### 保留不变

`scan_all_tools`、`scan_tool`、`get_skill_detail`、`save_skill_content`、`get_tool_configs`、`update_tool_config`、`reveal_in_finder`

#### `get_skill_detail` 调整

`push_status` 里的 `deployed` 字段语义从"是否 pushed"变为"是否 enabled（有 symlink）"。数据结构不变，前端无需改字段名。

---

## 前端设计

### API 层 (`src/lib/api.ts`)

```typescript
// 废弃
export function pushSkill(...)

// 新增/替换
export function enableSkill(skillName: string, targets: Tool[]): Promise<EnableResult[]>
export function disableSkill(skillName: string, targets: Tool[]): Promise<void>
export function deleteSkill(skillName: string): Promise<void>
export function migrateHub(newPath: string): Promise<MigrateReport>
export function syncSkills(): Promise<SyncReport>
export function verifySkills(): Promise<VerifyReport>
```

### SkillDetailPage 变更

右侧面板操作区改造：

| 当前 | 改为 |
|------|------|
| Push 按钮（每个工具一个） | Enable/Disable 切换：已 enabled 显示开关状态 |
| Push All 按钮 | Enable All / Disable All |
| Disable/Enable 按钮（文件重命名语义） | 移除 |
| Delete 按钮 | 保留，调用新的 `deleteSkill(name)` |

### SkillsPage 变更

顶部操作栏新增 **Sync** 按钮（与 Scan 并排）：
- 点击调用 `syncSkills()`
- 完成后显示 SyncReport
- 自动触发 rescan 刷新列表

### Settings 弹窗

SkillsPage 顶部操作栏加 Settings 图标，点击弹出轻量弹窗：
- skillshub 路径输入框 + 确认按钮
- 修改时调用 `migrateHub(newPath)`
- 完成后展示 MigrateReport

---

## 影响范围

### 新增文件
- `src-tauri/src/features/skills/hub.rs`

### 修改文件
- `src-tauri/src/features/skills/mod.rs` — 加 `hub`，移除 `pusher`
- `src-tauri/src/features/skills/commands.rs` — 替换命令
- `src-tauri/src/features/skills/types.rs` — 加 MigrateReport/SyncReport/VerifyReport/EnableResult
- `src/lib/api.ts` — 替换 API 函数
- `src/lib/types.ts` — 加前端类型
- `src/features/skills/pages/SkillDetailPage.tsx` — enable/disable 切换 UI
- `src/features/skills/pages/SkillsPage.tsx` — Sync 按钮 + Settings 弹窗
- `src/features/skills/hooks/useSkillDetail.ts` — enable/disable/delete 方法

### 删除文件
- `src-tauri/src/features/skills/pusher.rs`
