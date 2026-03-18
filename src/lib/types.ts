/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 Tool, SkillMeta, SkillDetail, EnableResult, MigrateReport, SyncReport, VerifyReport, ProviderProfile, ProvidersConfig 等类型
 * [POS]: 全局 TS 类型，镜像 Rust 后端数据结构
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
export type Tool = "claude_code" | "codex" | "cursor" | "trae"

export type SkillFormat = "skill_md" | "agents_md" | "cursor_mdc" | "trae_rules"

export type Status = "active" | "disabled"

export interface SkillMeta {
  id: string
  name: string
  description: string
  source_tool: Tool
  file_path: string
  format: SkillFormat
  status: Status
  version: string | null
  modified_at: number | null
  content_hash: string
}

export interface PushTarget {
  tool: Tool
  deployed: boolean
  target_path: string | null
}

export interface SkillDetail {
  meta: SkillMeta
  content: string
  push_status: PushTarget[]
  dir_name: string | null
}

export type PushResult =
  | { success: { path: string } }
  | { already_exists: { path: string } }
  | { error: { message: string } }

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

export const TOOL_LABELS: Record<Tool, string> = {
  claude_code: "Claude Code",
  codex: "Codex",
  cursor: "Cursor",
  trae: "Trae",
}

// ---- Provider 供应商管理 ----

export type ProviderType = "official" | "custom"

export interface ProviderProfile {
  id: string
  name: string
  tool: Tool
  provider_type: ProviderType
  base_url?: string
  api_key?: string
}

export interface ProvidersConfig {
  providers: ProviderProfile[]
  active: Record<string, string>
}
