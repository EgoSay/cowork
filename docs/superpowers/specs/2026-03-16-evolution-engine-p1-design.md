# AI 协作进化引擎 — Phase 1: 感知闭环

## 定位

进化引擎的数据基础层。从 Claude Code 会话历史中提取项目和会话元数据，
建立"看见 → 反思 → 标记 → 回顾"的最小完整闭环。

设计哲学：标注行为本身即刻意练习（历史学），晨间焦点是仪式不是报表（人类学），
时间分配用比例不用精确值（统计学）。

## 范围

**IN**
- Claude Code 项目扫描（`~/.claude/projects/`）
- 会话元数据解析（首条消息、时间戳、消息数、轮次数）
- 会话标注系统（高效 / 踩坑 / 模板，一键标注）
- 晨间焦点面板（昨日数据 + 时间分配比例 + 留白）
- 会话闪卡（检测新会话 → 弹出摘要 + 标注入口）
- 按项目 / 标签筛选回顾
- 复用 Usage 模块的 Token 数据（通过 project_id 关联）

**OUT**
- Codex / Cursor / Trae 支持（P4）
- 自动会话分类 feature/debug/refactor（P2）
- AI 驱动的模式识别和经验提取（P3）
- 实时会话监测 file watcher（P4）
- 完整对话内容渲染

---

## 1. 数据架构

### 1.1 数据源：`~/.claude/projects/`

```
~/.claude/projects/
  {encoded-path}/                     # 编码规则: / → -, . → -
    ├── {uuid}.jsonl                  # 主会话
    ├── {uuid}.jsonl.wakatime         # WakaTime 元数据（忽略）
    └── {uuid}/
        └── subagents/
            └── agent-{id}.jsonl      # 子代理会话（不独立展示，计入父会话）
```

**目录名解码**：编码规则为 `/` → `-`，`.` → `-`（不可逆）。
解码策略：取最后一段作为项目显示名（如 `feat-project`），
完整路径存储为 `dir_name` 供调试，不做完美反解。

### 1.2 JSONL 事件结构（已知，复用现有 parser 知识）

```jsonl
{"type":"system","message":{"content":"..."},"timestamp":"2026-03-11T14:00:00+08:00"}
{"type":"user","message":{"content":"hello"},"timestamp":"2026-03-11T14:01:00+08:00"}
{"type":"assistant","message":{"id":"msg_01","model":"claude-opus-4-6","usage":{...}},"timestamp":"2026-03-11T14:02:00+08:00"}
```

### 1.3 新增数据结构

**ProjectMeta** — 项目概览

| 字段 | 类型 | 来源 |
|------|------|------|
| id | String | `path_to_id(dir_path)` |
| name | String | 目录名最后一段（如 `feat-project`） |
| dir_name | String | 完整编码目录名 |
| dir_path | String | 磁盘绝对路径 |
| session_count | usize | `.jsonl` 文件数（不含 subagents） |
| last_active | i64 | 最新会话的 `ended_at` |
| total_sessions_duration_secs | u64 | 所有会话时长之和 |

**SessionMeta** — 会话摘要

| 字段 | 类型 | 来源 |
|------|------|------|
| id | String | JSONL 文件名中的 UUID |
| project_id | String | 父 ProjectMeta.id |
| title | String | 第一条 `user` 消息内容（截断 120 字符） |
| started_at | i64 | 第一条消息的 timestamp（Unix ms） |
| ended_at | i64 | 最后一条消息的 timestamp |
| duration_secs | u64 | `ended_at - started_at` |
| message_count | usize | 总消息数 |
| user_message_count | usize | `type=user` 的消息数 |
| turn_count | usize | user→assistant 轮次对数 |
| has_subagents | bool | 是否存在 subagents 子目录 |

**SessionAnnotation** — 用户标注

| 字段 | 类型 | 说明 |
|------|------|------|
| tags | Vec\<String\> | 多选：`efficient` / `pitfall` / `template` |
| note | Option\<String\> | 可选备注（闪卡追问的回答） |
| created_at | i64 | 标注时间 |

### 1.4 持久化

**标注文件**：`~/.cowork/annotations.toml`

```toml
[sessions."a1b2c3d4"]
tags = ["efficient"]
note = "一次命中，prompt 结构值得复用"
created_at = 1710000000

[sessions."e5f6g7h8"]
tags = ["pitfall"]
note = "CSS 反复修正 5 轮"
created_at = 1710100000
```

**缓存文件**：`~/.cowork/projects_cache.json`
- 每个 JSONL 文件的 mtime + 解析结果
- 增量扫描：仅重新解析 mtime 变化的文件

---

## 2. 后端设计（Rust）

### 2.1 模块结构

```
src-tauri/src/features/projects/
├── mod.rs              # 模块声明
├── types.rs            # ProjectMeta, SessionMeta, SessionAnnotation
├── scanner.rs          # 项目目录扫描 + 会话元数据提取
├── annotations.rs      # 标注 CRUD（读写 annotations.toml）
└── commands.rs         # Tauri IPC commands
```

### 2.2 Scanner 核心逻辑

```
scan_projects():
  1. 列举 ~/.claude/projects/ 下所有子目录
  2. 对每个子目录：
     a. 列举 *.jsonl 文件（排除 subagents/ 下的）
     b. 对每个文件：检查 cache → mtime 一致则跳过，否则解析
     c. 解析策略：
        - 正向读取：逐行扫描直到找到第一条 type=user 的消息 → title
        - 正向读取：统计所有行的 type → message_count, user_message_count, turn_count
        - 正向读取：记录第一条消息的 timestamp → started_at
        - 正向读取：记录最后一条消息的 timestamp → ended_at
        - 检查 {uuid}/subagents/ 目录是否存在 → has_subagents
     d. 缓存结果（关联 mtime）
  3. 聚合为 ProjectMeta 列表，按 last_active 降序
```

**性能约束**：
- JSONL 文件可能很大（数 MB），但我们需要完整扫描来统计消息数
- 优化：逐行流式读取（`BufReader::lines()`），不加载整个文件到内存
- 增量缓存：`projects_cache.json` 按 file_path + mtime 索引
- 首次扫描可能慢（几秒），后续增量扫描亚秒级

### 2.3 Annotations 核心逻辑

```
annotate_session(session_id, tags, note):
  1. 读取 ~/.cowork/annotations.toml
  2. 插入/更新 [sessions.{session_id}]
  3. 写回文件

get_annotations() -> HashMap<String, SessionAnnotation>:
  1. 读取 annotations.toml
  2. 返回全部标注（前端做关联）

remove_annotation(session_id):
  1. 删除对应条目
  2. 写回
```

### 2.4 Tauri Commands

```rust
#[tauri::command]
async fn scan_projects() -> Result<Vec<ProjectMeta>, String>

#[tauri::command]
async fn get_project_sessions(dir_path: String) -> Result<Vec<SessionMeta>, String>

#[tauri::command]
async fn annotate_session(
    session_id: String,
    tags: Vec<String>,
    note: Option<String>,
) -> Result<(), String>

#[tauri::command]
async fn get_annotations() -> Result<HashMap<String, SessionAnnotation>, String>

#[tauri::command]
async fn remove_annotation(session_id: String) -> Result<(), String>
```

---

## 3. 前端设计（React）

### 3.1 模块结构

```
src/features/projects/
├── pages/
│   └── ProjectsPage.tsx       # 主页面（晨间焦点 + 项目列表 + 会话列表）
├── components/
│   ├── MorningFocus.tsx        # 晨间焦点面板
│   ├── ProjectCard.tsx         # 项目卡片
│   ├── SessionCard.tsx         # 会话卡片（含标注按钮）
│   ├── FlashCard.tsx           # 新会话弹出闪卡
│   ├── TagFilter.tsx           # 标签筛选栏
│   └── TimeDistribution.tsx    # 时间分配比例条
└── hooks/
    └── useProjects.ts          # 核心 reducer hook
```

### 3.2 useProjects Hook（单真相源）

```typescript
interface State {
  projects: ProjectMeta[]
  sessions: SessionMeta[]          // 当前选中项目的会话列表
  annotations: Record<string, SessionAnnotation>
  selectedProject: ProjectMeta | null
  search: string
  tagFilter: string[]              // 空 = 全部
  loading: boolean
  error: string | null
  newSession: SessionMeta | null   // 闪卡用：检测到的新会话
}
```

**派生数据（useMemo，不 useState）**：
- `filteredProjects`：按 search 过滤 + 按 last_active 排序
- `filteredSessions`：按 tagFilter 过滤会话
- `timeDistribution`：按标签统计会话数占比
- `morningFocusData`：昨日会话数、总轮次、标注分布

### 3.3 页面布局

```
┌─────────────────────────────────────────────────┐
│ Morning Focus（可折叠）                           │
│ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────────────┐│
│ │昨日    │ │平均    │ │标注    │ │ 时间分配      ││
│ │会话: 7 │ │轮次:2.8│ │高效: 4 │ │ ■■■■□□□□□□  ││
│ └───────┘ └───────┘ └───────┘ └───────────────┘│
│                     （50% 留白空间）               │
├─────────────────────────────────────────────────┤
│ [搜索框]                    [标签筛选: 全部 ▾]    │
├──────────────┬──────────────────────────────────┤
│ 项目列表      │ 会话列表（选中项目后展示）         │
│              │                                  │
│ ▸ cowork     │  "实现项目扫描器"                  │
│   7 会话     │  3/16 14:23  18msg  3轮  [高效]   │
│   最近 2h前   │                                  │
│              │  "修复 CSS 布局"                   │
│ ▸ my-app     │  3/16 10:15  32msg  8轮  [踩坑]   │
│   3 会话     │                                  │
│   最近 1d前   │  "添加搜索功能"                   │
│              │  3/15 16:00  12msg  2轮           │
├──────────────┴──────────────────────────────────┤
```

### 3.4 交互：会话闪卡

**触发时机**：Projects 页面获得焦点时（re-entry），对比缓存的会话列表，
检测到新增会话 → 弹出闪卡。

**闪卡内容**：
```
┌─────────────────────────────────────────┐
│  cowork / feat-project                  │
│  "实现项目浏览器的后端扫描器"             │
│                                         │
│  14:23 — 15:47   18 msg   3 轮          │
│                                         │
│  第 3 轮修正是否可以避免？               │ ← 刻意练习触发
│  ┌─────────────────────────────────┐    │
│  │ 备注 (可选)                      │    │
│  └─────────────────────────────────┘    │
│                                         │
│  [✓ 高效]  [✗ 踩坑]  [📋 模板]  [跳过]  │
└─────────────────────────────────────────┘
```

**关键设计**：
- 闪卡追问"第 N 轮修正是否可以避免？"（turn_count > 3 时显示）
- 标注按钮支持多选（可以同时标"踩坑"和"模板"）
- "跳过"不标注也保存——不强制，零摩擦
- 备注框可选，不展开除非用户点击

### 3.5 交互：晨间焦点

**设计原则**：50% 留白——激活默认模式网络，让用户自己产生连接。

**数据项（仅 3 个数字 + 1 个比例条）**：
- 昨日会话数
- 昨日平均轮次
- 昨日标注分布（N 个高效 / N 个踩坑）
- 时间分配比例条（按项目或按标签，展示会话数占比）

**不展示**：Token 数据（已有 Usage 模块）、精确时长、趋势图（P2）。

### 3.6 时间分配比例

基于**会话数量占比**，不基于精确时长：

```
按项目:  cowork ████████░░ 60%   my-app ████░░░░░░ 30%   other █░ 10%
按标签:  高效 ██████░░░░ 50%   踩坑 ████░░░░░░ 30%   未标注 ██░░ 20%
```

可切换维度：按项目 / 按标签。默认按标签（更有进化洞察价值）。

---

## 4. App.tsx 集成

延续 keep-alive + visited 懒挂载模式：

```typescript
// App.tsx 新增
import { ProjectsPage } from "@/features/projects/pages/ProjectsPage"

// 在 render 中
{visited.has("projects") && (
  <div className={activeModule === "projects" ? "contents" : "hidden"}>
    <ProjectsPage active={activeModule === "projects"} />
  </div>
)}
```

ModuleNav 中 `projects` 的 `enabled` 设为 `true`。

---

## 5. 与 Usage 模块的关系

**不复制，只关联**：
- Projects 模块不解析 Token 数据
- Usage 模块已有完整的 DailyRecord（按日/模型聚合）
- 未来 P2 可通过 project_id 维度扩展 Usage，但 P1 不改动 Usage

**P1 中 Token 数据不出现在 Projects 页面**。
晨间焦点只展示会话数、轮次、标注——这些是 Projects 模块自己的数据。
Token 相关洞察留在 Usage 模块。

---

## 6. 测试策略

### 后端

| 测试 | 覆盖 |
|------|------|
| `scanner::parse_session_meta` | 从 JSONL 字符串提取 SessionMeta |
| `scanner::parse_session_meta_empty` | 空文件 / 损坏文件返回 None |
| `scanner::scan_projects_dir` | tempdir 模拟项目结构，验证 ProjectMeta 聚合 |
| `scanner::incremental_cache` | mtime 未变 → 跳过解析 |
| `annotations::roundtrip` | 写入 → 读取 → 验证一致 |
| `annotations::update_existing` | 覆盖已有标注 |
| `annotations::remove` | 删除后不存在 |

### 前端

| 测试 | 覆盖 |
|------|------|
| `useProjects` reducer | 各 action type 状态转换 |
| `filteredSessions` | 按标签过滤正确性 |
| `timeDistribution` | 会话数占比计算 |

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 大 JSONL 文件扫描慢 | 首次加载 >3s | BufReader 流式 + 增量缓存 |
| 用户不标注 | 进化闭环断裂 | 闪卡自动弹出 + 一键操作 + 不标注不影响基础功能 |
| 目录名解码不准 | 项目显示名不直观 | 取最后一段 + 保留完整 dir_name 可查 |
| 标注文件并发写入 | 数据损坏 | 单线程写入 + Mutex（延续 Provider 模块模式） |

---

## 8. 全局路线图摘要

| Phase | 目标 | 依赖 |
|-------|------|------|
| **P1（当前）** | 感知闭环：看见 + 标记 + 回顾 | 无 |
| P2 | 认知深化：自动分类 + 模式识别 + 周报 + 进化轨迹 | P1 标注数据 |
| P3 | 进化引擎：AI 经验提取 + 知识沉淀 + 上下文延续 | P2 模式数据 + Provider 模块 |
| P4 | 协作增强：实时监测 + 智能教练 + 多工具支持 | P3 经验库 |
