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
- **路径安全由构造保证**：所有 hub 函数的参数是 skill 目录名（非任意路径），实际路径始终从 `config.get_skillshub_dir()` 和 `config.get_skills_dir(tool)` 派生，杜绝路径注入

---

## 后端设计

### Config 扩展 (`src-tauri/src/config.rs`)

新增方法：

```rust
impl AppConfig {
    /// 返回 skillshub 目录的展开路径
    pub fn get_skillshub_dir(&self) -> PathBuf {
        let dir = self.tools.get("skillshub")
            .map(|t| t.skills_dir.as_str())
            .unwrap_or("~/.skillshub");
        expand_tilde(dir)
    }
}
```

### 新模块：`src-tauri/src/features/skills/hub.rs`

#### 路径解析约定

所有函数接受 `skill_dir_name: &str`（即 skill 在 skillshub 里的目录名），内部通过 `config.get_skillshub_dir().join(skill_dir_name)` 构造完整路径。不接受任意用户路径。

**例外**：`migrate` 操作的是 hub 目录本身而非单个 skill，因此接受 `old_path`/`new_path`。但 command 层的 `migrate_hub` 从 config 读取 old_path，用户只提供 new_path，安全边界仍然成立。

#### 核心函数

```rust
/// 在 tool 的 skills 目录创建指向 skillshub 的 symlink
/// 解析: skillshub_dir.join(skill_dir_name) → tool_dir.join(skill_dir_name)
pub fn enable(skill_dir_name: &str, tool: &Tool, config: &AppConfig) -> Result<EnableResult>

/// 删除 tool 目录里的 symlink（skill 仍在 skillshub）
/// 安全: 仅删除 is_symlink() == true 的条目
pub fn disable(skill_dir_name: &str, tool: &Tool, config: &AppConfig) -> Result<()>

/// 从 skillshub 删除 skill 目录 + 清理所有工具里指向它的 symlink
pub fn delete(skill_dir_name: &str, config: &AppConfig) -> Result<()>

/// 复制 old → new，重建所有 symlink，校验一致性
pub fn migrate(old_path: &Path, new_path: &Path, config: &AppConfig) -> Result<MigrateReport>

/// 扫描各工具目录，找到非 symlink skill → 复制到 skillshub → 替换为 symlink
/// 原子顺序: copy → rename original to .bak → create symlink → verify → delete .bak
pub fn sync(config: &AppConfig) -> Result<SyncReport>

/// 遍历所有 symlink，检查：指向存在？目标在 skillshub 内？
pub fn verify(config: &AppConfig) -> Result<VerifyReport>
```

#### 返回类型

```rust
pub enum EnableResult {
    Success { path: String },
    AlreadyEnabled { path: String },
}
// 实际错误通过 Result::Err 传播，不在枚举中混用

pub struct MigrateReport {
    pub copied: Vec<String>,                    // 成功复制的 skill 名
    pub symlinks_updated: Vec<(Tool, String)>,  // (工具, skill名)
    pub errors: Vec<String>,                    // 失败项
    pub verified: bool,                         // 最终校验是否通过
}

pub struct SyncReport {
    pub imported: Vec<(Tool, String)>,   // 从哪个工具导入了什么
    pub skipped: Vec<(Tool, String, String)>,  // (工具, skill名, 原因) 如名称冲突
    pub errors: Vec<String>,
}

pub struct VerifyReport {
    pub ok: Vec<(Tool, String)>,                // 健康的 symlink
    pub broken: Vec<(Tool, String, String)>,     // (工具, skill名, 原因)
}
```

#### 关键行为

- **enable**：`symlink(skillshub_dir/name, tool_dir/name)`，已存在则返回 AlreadyEnabled
- **disable**：检查 target `is_symlink()` 才删除，防止误删真实文件
- **delete**：先遍历所有工具目录清理 symlink，再删 skillshub 里的目录
- **migrate**：复制 old → new → 遍历所有工具目录找指向 old 的 symlink → 删除旧 symlink → 创建新 symlink → 调 `verify()` 校验
- **sync 原子顺序**：
  1. 遍历各工具目录，找 `is_symlink() == false` 且含 SKILL.md 的目录
  2. 复制到 skillshub（若名称冲突则记入 `skipped`）
  3. 将原目录重命名为 `.bak`
  4. 创建 symlink 指向 skillshub
  5. 验证 symlink 可达
  6. 成功则删除 `.bak`，失败则从 `.bak` 恢复
- **verify**：遍历所有工具目录的 symlink，检查 `read_link()` 目标是否存在且在 skillshub 内
- **通过 symlink 写入 skill 内容**：`save_skill_content` 写入 symlink 路径时，OS 自动跟随到 skillshub 真实文件，这是预期行为

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
// skill_name 是 skillshub 目录名，循环调 hub::enable
// fail-fast: 任一 tool 出错则整个命令返回 Err，前序成功的 symlink 保留

#[tauri::command]
fn disable_skill(skill_name: String, targets: Vec<Tool>) -> Result<()>

#[tauri::command]
fn delete_skill(skill_name: String) -> Result<()>
// skill_name 是 skillshub 目录名

#[tauri::command]
fn migrate_hub(new_path: String) -> Result<MigrateReport>
// 读当前 config 取 old_path → hub::migrate → 更新 config → 保存

#[tauri::command]
fn sync_skills() -> Result<SyncReport>

#[tauri::command]
fn verify_skills() -> Result<VerifyReport>
```

#### 保留不变

`scan_all_tools`、`scan_tool`、`get_skill_detail`、`save_skill_content`、`get_tool_configs`、`update_tool_config`、`reveal_in_finder`

#### `get_skill_detail` 调整

- `push_status` 里的 `deployed` 字段语义从"是否 pushed"变为"是否 enabled（有 symlink）"。数据结构不变
- `skill_dir_name()` 工具函数的 import 路径从 `super::pusher` 改为 `super::hub`

---

## 前端设计

### 类型 (`src/lib/types.ts`)

```typescript
export type EnableResult =
  | { success: { path: string } }
  | { already_enabled: { path: string } }

export interface MigrateReport {
  copied: string[]
  symlinks_updated: [Tool, string][]
  errors: string[]
  verified: boolean
}

export interface SyncReport {
  imported: [Tool, string][]
  skipped: [Tool, string, string][]
  errors: string[]
}

export interface VerifyReport {
  ok: [Tool, string][]
  broken: [Tool, string, string][]
}
```

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

`useSkillDetail` hook 的 `push(targets)` 改为 `enable(targets)` / `disable(targets)`，参数从 `skill.file_path` 改为从 `SkillDetail` 中获取的目录名。

### SkillsPage 变更

顶部操作栏新增 **Sync** 按钮（与 Scan 并排）：
- 点击调用 `syncSkills()`
- 完成后显示 SyncReport（toast 或内联消息）
- 自动触发 rescan 刷新列表

### Settings 弹窗

SkillsPage 顶部操作栏加 Settings 图标，点击弹出轻量弹窗：
- skillshub 路径输入框（当前值 + 确认按钮）
- 修改时调用 `migrateHub(newPath)`
- 完成后展示 MigrateReport
- 弹窗底部显示 **Verify** 按钮，调用 `verifySkills()`，展示 VerifyReport（健康/损坏 symlink 列表）

---

## 影响范围

### 新增文件
- `src-tauri/src/features/skills/hub.rs`

### 修改文件
- `src-tauri/src/config.rs` — 新增 `get_skillshub_dir()` 方法
- `src-tauri/src/features/skills/mod.rs` — 加 `hub`，移除 `pusher`
- `src-tauri/src/features/skills/commands.rs` — 替换命令，更新 `skill_dir_name` import
- `src-tauri/src/features/skills/types.rs` — 加 MigrateReport/SyncReport/VerifyReport/EnableResult
- `src/lib/api.ts` — 替换 API 函数
- `src/lib/types.ts` — 加前端类型（EnableResult, MigrateReport, SyncReport, VerifyReport）
- `src/features/skills/pages/SkillDetailPage.tsx` — enable/disable 切换 UI
- `src/features/skills/pages/SkillsPage.tsx` — Sync 按钮 + Settings 弹窗
- `src/features/skills/hooks/useSkillDetail.ts` — enable/disable/delete 方法

### 删除文件
- `src-tauri/src/features/skills/pusher.rs`

---

## v1 范围外（已知但显式推迟）

- **migrate 进度反馈**：v1 同步返回 MigrateReport，文件复制通常很快。若未来 skill 数量大幅增长再考虑 Tauri event stream
- **Windows 支持**：symlink 依赖 `std::os::unix::fs::symlink`，仅 Unix。当前项目 macOS only
- **Scanner 的 skillshub 扫描行为**：`scan_all` 中 skillshub 仍作为 ClaudeCode 源扫描，不改变现有行为
