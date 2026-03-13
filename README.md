# CoWork

AI 编码工具指挥中心。统一管理 Claude Code / Codex / Cursor / Trae 的 Skills、用量监控、API 供应商切换。

## 技术栈

Rust + Tauri 2 + React 18 + TypeScript + Tailwind v4

## 功能

### Skills 管理

- **统一扫描** — 自动发现四个 AI 编码工具的 Skills 文件（SKILL.md / AGENTS.md / .mdc 等）
- **内容去重** — 基于 SHA-256 哈希，相同内容只保留一份
- **详情预览** — 查看 Skill 元信息、来源工具、完整内容
- **跨工具推送** — 将 Skill 通过 symlink 同步到其他工具（`~/.skillshub/` 为中心）

### Usage 用量监控

- **双工具解析** — 解析 Claude Code 和 Codex 的 session JSONL 日志
- **四维统计** — input / output / cache_read / cache_write 统一口径
- **时间筛选** — Today / 7D / 30D 预设 + 自定义日期范围
- **可视化仪表盘** — 摘要卡片、每日柱状图、模型分布表

### Providers 供应商管理

- **多供应商配置** — 添加 / 编辑 / 删除 API 供应商（name + base_url + api_key）
- **一键切换** — 修改 `~/.claude/settings.json` 的 env 段实现供应商切换

## 支持的工具

| 工具 | Skills 格式 | 默认扫描路径 |
|------|------------|-------------|
| Claude Code | `SKILL.md` (YAML frontmatter) | `~/.claude/skills/` |
| Codex | `AGENTS.md` | `~/.codex/skills/` |
| Cursor | `.mdc` | `~/.cursor/skills/` |
| Trae | `.md` | `~/.trae/skills/` |

## 快速开始

### 前置依赖

- [Rust](https://rustup.rs/) (>= 1.85.0)
- [Node.js](https://nodejs.org/) (>= 20)
- [pnpm](https://pnpm.io/) (>= 10)

### 安装 & 启动

```bash
pnpm install
pnpm dev
```

### 构建

```bash
pnpm build
```

### 测试

```bash
cd src-tauri && cargo test
pnpm typecheck
```

## 配置

**工具路径** — `~/.cowork/config.toml`

```toml
[tools.claude_code]
skills_dir = "~/.claude/skills"
scan_patterns = ["*/SKILL.md"]

[tools.codex]
skills_dir = "~/.codex/skills"
scan_patterns = ["AGENTS.md"]

[tools.cursor]
skills_dir = "~/.cursor/skills"
scan_patterns = ["*.mdc"]

[tools.trae]
skills_dir = "~/.trae/skills"
scan_patterns = ["*.md"]
```

**API 供应商** — `~/.cowork/providers.toml`

## 架构

```
src/                              # React 前端
├── components/layout/            # AppShell, TitleBar, ModuleNav
├── features/
│   ├── skills/                   # Skills 扫描 + 推送
│   ├── usage/                    # 用量监控仪表盘
│   └── providers/                # 供应商管理
└── lib/                          # 类型定义 + Tauri IPC 封装

src-tauri/src/                    # Rust 后端
├── features/
│   ├── skills/
│   │   ├── scanner/              # 四工具扫描器 (Scanner trait)
│   │   └── pusher/               # symlink 推送
│   ├── usage/
│   │   └── parser/               # Claude Code + Codex JSONL 解析
│   └── providers/                # 供应商 CRUD + settings.json 写入
└── shared/                       # 公共工具 (fs_utils)
```

## License

MIT
