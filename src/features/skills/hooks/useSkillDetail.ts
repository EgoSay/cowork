/**
 * [INPUT]: 依赖 @/lib/api 的 skill 操作函数, @/lib/types
 * [OUTPUT]: 对外提供 useSkillDetail hook（加载、推送、停用、删除）
 * [POS]: skills hooks 的详情页状态管理
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useState } from "react"
import { getSkillDetail, pushSkill, disableSkill, enableSkill, deleteSkill, revealInFinder } from "@/lib/api"
import type { SkillDetail, SkillMeta, Tool, PushResult } from "@/lib/types"

export function useSkillDetail(skill: SkillMeta, allSkills: SkillMeta[]) {
  const [detail, setDetail] = useState<SkillDetail | null>(null)
  const [loading, setLoading] = useState(true)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const d = await getSkillDetail(skill.id, allSkills)
      setDetail(d)
    } catch (e) {
      console.error("Failed to load skill detail:", e)
    }
    setLoading(false)
  }, [skill.id, allSkills])

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

  return { detail, loading, push, disable, enable, remove, reveal, reload: load }
}
