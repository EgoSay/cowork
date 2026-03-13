# CoWork

AI 编码工具的 Skills 指挥中心。统一扫描、浏览、推送 Claude Code / Codex / Cursor / Trae 四大工具的 Skills。

## 技术栈

Rust + Tauri 2 + React 18 + TypeScript + Tailwind v4

## 功能

- **统一扫描** — 自动发现四个 AI 编码工具的 Skills 文件（SKILL.md / AGENTS.md / .mdc 等）
- **内容去重** — 基于 SHA-256 哈希，相同内容只保留一份
- **详情预览** — 查看 Skill 元信息、来源工具、完整内容
- **跨工具推送** — 将 Skill 从一个工具复制到另一个工具（文件复制，非软链接）
- **纯文件系统** — 无数据库，原格式存储，内存索引

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

CoWork 使用 `~/.cowork/config.toml` 存储工具路径配置：

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

## 架构

```
src/                          # React 前端
├── components/layout/        # AppShell, TitleBar, ModuleNav
├── features/skills/          # Skills 模块 (pages, components, hooks)
└── lib/                      # 类型定义 + Tauri IPC 封装

src-tauri/src/                # Rust 后端
├── features/skills/
│   ├── scanner/              # 四工具扫描器 (Scanner trait)
│   ├── pusher/               # 跨工具推送
│   └── commands/             # Tauri 命令层
└── shared/                   # 公共工具 (fs_utils)
```

## License

MIT
