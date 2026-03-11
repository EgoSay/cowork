/**
 * [INPUT]: 依赖 @/lib/api::scanAllTools, @/lib/types
 * [OUTPUT]: 对外提供 useSkills hook（扫描、过滤、搜索）
 * [POS]: skills hooks 的核心，管理列表状态
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useReducer } from "react"
import { scanAllTools } from "@/lib/api"
import type { SkillMeta, Tool } from "@/lib/types"

interface State {
  skills: SkillMeta[]
  filter: Tool | "all"
  search: string
  loading: boolean
  error: string | null
}

type Action =
  | { type: "SET_LOADING" }
  | { type: "SET_SKILLS"; skills: SkillMeta[] }
  | { type: "SET_ERROR"; error: string }
  | { type: "SET_FILTER"; filter: Tool | "all" }
  | { type: "SET_SEARCH"; search: string }

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "SET_LOADING":
      return { ...state, loading: true, error: null }
    case "SET_SKILLS":
      return { ...state, skills: action.skills, loading: false }
    case "SET_ERROR":
      return { ...state, error: action.error, loading: false }
    case "SET_FILTER":
      return { ...state, filter: action.filter }
    case "SET_SEARCH":
      return { ...state, search: action.search }
  }
}

export function useSkills() {
  const [state, dispatch] = useReducer(reducer, {
    skills: [],
    filter: "all",
    search: "",
    loading: true,
    error: null,
  })

  const scan = useCallback(async () => {
    dispatch({ type: "SET_LOADING" })
    try {
      const skills = await scanAllTools()
      dispatch({ type: "SET_SKILLS", skills })
    } catch (e) {
      dispatch({ type: "SET_ERROR", error: String(e) })
    }
  }, [])

  useEffect(() => { scan() }, [scan])

  const filtered = state.skills.filter((s) => {
    if (state.filter !== "all" && s.source_tool !== state.filter) return false
    if (state.search) {
      const q = state.search.toLowerCase()
      return (
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q)
      )
    }
    return true
  })

  const toolCounts = state.skills.reduce(
    (acc, s) => {
      acc[s.source_tool] = (acc[s.source_tool] || 0) + 1
      return acc
    },
    {} as Record<string, number>
  )

  return {
    skills: filtered,
    allSkills: state.skills,
    toolCounts,
    totalCount: state.skills.length,
    filter: state.filter,
    search: state.search,
    loading: state.loading,
    error: state.error,
    setFilter: (f: Tool | "all") => dispatch({ type: "SET_FILTER", filter: f }),
    setSearch: (s: string) => dispatch({ type: "SET_SEARCH", search: s }),
    rescan: scan,
  }
}
