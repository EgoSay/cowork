/**
 * [INPUT]: 依赖 @/lib/api 的 skill 操作函数（含 saveSkillContent）, @/lib/types
 * [OUTPUT]: 对外提供 useSkillDetail hook（加载、推送、停用、删除、保存内容）
 * [POS]: skills hooks 的详情页状态管理
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useState } from "react"
import { getSkillDetail, pushSkill, disableSkill, enableSkill, deleteSkill, revealInFinder, saveSkillContent } from "@/lib/api"
import type { SkillDetail, SkillMeta, Tool, PushResult } from "@/lib/types"

export function useSkillDetail(skill: SkillMeta) {
  const [detail, setDetail] = useState<SkillDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const d = await getSkillDetail(skill)
      setDetail(d)
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      console.error("Failed to load skill detail:", msg)
      setError(msg)
    }
    setLoading(false)
  }, [skill.id])

  useEffect(() => { load() }, [load])

  const push = async (targets: Tool[]): Promise<PushResult[]> => {
    return pushSkill(skill.file_path, targets)
  }

  const disable = async () => {
    await disableSkill(skill.file_path)
    await load()
  }

  const enable = async () => {
    await enableSkill(skill.file_path)
    await load()
  }

  const remove = async () => {
    await deleteSkill(skill.file_path)
  }

  const reveal = async () => {
    await revealInFinder(skill.file_path)
  }

  const save = async (content: string) => {
    await saveSkillContent(skill.file_path, content)
    await load()
  }

  return { detail, loading, error, push, disable, enable, remove, reveal, save, reload: load }
}
