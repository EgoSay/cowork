/**
 * [INPUT]: 依赖 @tauri-apps/api/core 的 invoke, ./types
 * [OUTPUT]: 对外提供所有 Tauri IPC 封装函数
 * [POS]: 前端 API 层，类型安全的 invoke 封装
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { invoke } from "@tauri-apps/api/core"
import type { SkillMeta, SkillDetail, PushResult, Tool, UsageData, ProvidersConfig } from "./types"

export async function scanAllTools(): Promise<SkillMeta[]> {
  return invoke<SkillMeta[]>("scan_all_tools")
}

export async function scanTool(tool: Tool): Promise<SkillMeta[]> {
  return invoke<SkillMeta[]>("scan_tool", { tool })
}

export async function getSkillDetail(
  meta: SkillMeta
): Promise<SkillDetail> {
  return invoke<SkillDetail>("get_skill_detail", { meta })
}

export async function pushSkill(
  filePath: string,
  targets: Tool[]
): Promise<PushResult[]> {
  return invoke<PushResult[]>("push_skill", { filePath, targets })
}

export async function disableSkill(filePath: string): Promise<void> {
  return invoke("disable_skill", { filePath })
}

export async function enableSkill(filePath: string): Promise<void> {
  return invoke("enable_skill", { filePath })
}

export async function deleteSkill(filePath: string): Promise<void> {
  return invoke("delete_skill", { filePath })
}

export async function revealInFinder(path: string): Promise<void> {
  return invoke("reveal_in_finder", { path })
}

export async function getUsageData(): Promise<UsageData> {
  return invoke<UsageData>("get_usage_data")
}

export async function getProviders(): Promise<ProvidersConfig> {
  return invoke<ProvidersConfig>("get_providers")
}

export async function switchProvider(toolKey: Tool, providerId: string): Promise<void> {
  return invoke("switch_provider", { toolKey, providerId })
}

export async function addProvider(
  id: string,
  name: string,
  tool: Tool,
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
