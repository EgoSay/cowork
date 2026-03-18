/**
 * [INPUT]: 依赖 @/lib/api 的 skill 操作函数, @/lib/types
 * [OUTPUT]: 对外提供 useSkillDetail hook（加载、启用、禁用、删除、保存内容）
 * [POS]: skills hooks 的详情页状态管理
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useState } from "react"
import { getSkillDetail, enableSkill, disableSkill, deleteSkill, revealInFinder, saveSkillContent } from "@/lib/api"
import type { SkillDetail, SkillMeta, Tool, EnableResult } from "@/lib/types"

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

  const enable = async (targets: Tool[]): Promise<EnableResult[]> => {
    if (!detail?.dir_name) throw new Error("Cannot resolve skill directory name")
    return enableSkill(detail.dir_name, targets)
  }

  const disable = async (targets: Tool[]) => {
    if (!detail?.dir_name) throw new Error("Cannot resolve skill directory name")
    await disableSkill(detail.dir_name, targets)
    await load()
  }

  const remove = async () => {
    if (!detail?.dir_name) throw new Error("Cannot resolve skill directory name")
    await deleteSkill(detail.dir_name)
  }

  const reveal = async () => {
    await revealInFinder(skill.file_path)
  }

  const save = async (content: string) => {
    await saveSkillContent(skill.file_path, content)
    await load()
  }

  return { detail, loading, error, enable, disable, remove, reveal, save, reload: load }
}
