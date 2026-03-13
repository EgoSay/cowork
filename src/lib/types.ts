/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 Tool, SkillMeta, SkillDetail, PushResult, DailyRecord, UsageData, ProviderProfile, ProvidersConfig 等类型
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
}

export type PushResult =
  | { success: { path: string } }
  | { already_exists: { path: string } }
  | { error: { message: string } }

export const TOOL_LABELS: Record<Tool, string> = {
  claude_code: "Claude Code",
  codex: "Codex",
  cursor: "Cursor",
  trae: "Trae",
}

// ── Usage (Token Statistics, 统一口径) ───────────────────

export interface DailyRecord {
  date: string
  tool: Tool
  model: string
  input_tokens: number
  output_tokens: number
  cache_read_tokens: number
  cache_write_tokens: number
}

export interface UsageData {
  records: DailyRecord[]
  scanned_from: string
  scanned_until: string
}

// ── Provider 供应商管理 ──────────────────────────────────

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
