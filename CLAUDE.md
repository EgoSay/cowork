# CoWork - AI 编码工具指挥中心
Rust + Tauri 2 + React 18 + TypeScript + Tailwind v4

<directory>
src/ - React 前端 (3子目录: components, features, lib)
src/components/layout/ - 壳组件: AppShell, TitleBar, ModuleNav
src/features/skills/ - Skills 管理功能模块 (3子目录: pages, components, hooks)
src/features/usage/ - Usage 监控仪表盘 (3子目录: pages, components, hooks)
src/features/providers/ - API 供应商管理模块 (3子目录: pages, components, hooks)
src/features/projects/ - 进化引擎 P1: 项目浏览器+会话标注+晨间焦点+热力图 (3子目录: pages, components, hooks)
src/lib/ - 全局类型 (types.ts) + Tauri IPC 封装 (api.ts)
src-tauri/ - Rust 后端 (1子目录: src)
src-tauri/src/ - Rust 源码入口
src-tauri/src/features/skills/ - Skills 扫描/推送/命令 (3子目录: scanner, pusher, commands)
src-tauri/src/features/skills/scanner/ - 四工具扫描器: claude_code, codex, cursor, trae
src-tauri/src/features/usage/ - Usage 数据聚合 (1子目录: parser)
src-tauri/src/features/usage/parser/ - 双工具 session JSONL 解析器: claude_code, codex
src-tauri/src/features/providers/ - 供应商后端: types, store, writer, commands
src-tauri/src/features/projects/ - 进化引擎 P1 后端: scanner+annotations+commands
src-tauri/src/shared/ - 共享工具: fs_utils (expand_tilde, hash_content, path_to_id)
</directory>

<config>
package.json - pnpm 包管理，Tauri CLI + React + Tailwind 依赖
tsconfig.json - TypeScript 配置，@/ 路径别名
vite.config.ts - Vite 构建，@tailwindcss/vite 插件，端口 1420
src-tauri/Cargo.toml - Rust 依赖: tauri 2, serde, serde_yaml, sha2, glob, dirs, toml, chrono
src-tauri/tauri.conf.json - Tauri 窗口配置, macOS Overlay 标题栏
</config>

## 架构决策
- 纯文件系统 + 内存索引，无数据库
- 原格式存储，不做 Skills 格式转换
- 符号链接推送：~/.skillshub/ 为中心，各工具 skills 目录通过 symlink 引用
- Monochrome 配色: #0a0a0a bg, #141414 card, #fafafa text
- features/ 模块化目录（前后端对称）
- Scanner trait 模式扫描四个工具
- `~/.cowork/config.toml` 存储工具路径配置
- `~/.cowork/providers.toml` 存储 API 供应商配置
- Provider 切换通过修改 `~/.claude/settings.json` env 段实现

## 开发规范
- `PATH="$HOME/.cargo/bin:$PATH"` 确保使用 rustup 管理的 Rust
- `pnpm tauri dev` 启动开发
- `cargo test` 在 src-tauri/ 目录运行
- `pnpm typecheck` 检查 TypeScript

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
