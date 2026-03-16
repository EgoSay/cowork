/**
 * [INPUT]: 依赖 @/lib/api::scanProjects/getAnnotations/annotateSession/removeAnnotation, @/lib/types, ../lib
 * [OUTPUT]: 对外提供 useProjects hook（单真相源，re-entry 静默刷新，flash card 检测，useMemo 派生数据）
 * [POS]: projects hooks 核心，管理项目列表/会话/标注状态
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useMemo, useReducer, useRef } from "react"
import {
  scanProjects,
  getAnnotations,
  annotateSession as apiAnnotate,
  removeAnnotation as apiRemove,
} from "@/lib/api"
import type { ProjectData, SessionMeta, SessionAnnotation } from "@/lib/types"
import {
  type DistributionItem,
  TAG_OPTIONS,
  localDateString,
  yesterdayString,
  computeDistribution,
} from "../lib"

// ── State ───────────────────────────────────────────

interface State {
  projectData: ProjectData[]
  annotations: Record<string, SessionAnnotation>
  selectedProjectId: string | null
  search: string
  tagFilter: string[]
  loading: boolean
  error: string | null
  flashSession: SessionMeta | null
}

type Action =
  | { type: "SET_LOADING" }
  | { type: "SET_DATA"; projectData: ProjectData[]; annotations: Record<string, SessionAnnotation> }
  | { type: "SET_ERROR"; error: string }
  | { type: "SELECT_PROJECT"; id: string | null }
  | { type: "SET_SEARCH"; search: string }
  | { type: "SET_TAG_FILTER"; tags: string[] }
  | { type: "SET_ANNOTATIONS"; annotations: Record<string, SessionAnnotation> }
  | { type: "SET_FLASH"; session: SessionMeta | null }
  | { type: "REFRESH_DATA"; projectData: ProjectData[]; annotations: Record<string, SessionAnnotation> }

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "SET_LOADING":
      return { ...state, loading: true, error: null }
    case "SET_DATA":
      return { ...state, projectData: action.projectData, annotations: action.annotations, loading: false }
    case "SET_ERROR":
      return { ...state, error: action.error, loading: false }
    case "SELECT_PROJECT":
      return { ...state, selectedProjectId: action.id }
    case "SET_SEARCH":
      return { ...state, search: action.search }
    case "SET_TAG_FILTER":
      return { ...state, tagFilter: action.tags }
    case "SET_ANNOTATIONS":
      return { ...state, annotations: action.annotations }
    case "SET_FLASH":
      return { ...state, flashSession: action.session }
    case "REFRESH_DATA":
      return { ...state, projectData: action.projectData, annotations: action.annotations }
  }
}

// ── Morning Focus 类型 ──────────────────────────────

export interface MorningFocus {
  sessionCount: number
  avgTurns: number
  efficient: number
  pitfall: number
}

// ── Hook ────────────────────────────────────────────

export function useProjects(active: boolean) {
  const [state, dispatch] = useReducer(reducer, {
    projectData: [],
    annotations: {},
    selectedProjectId: null,
    search: "",
    tagFilter: [],
    loading: true,
    error: null,
    flashSession: null,
  })

  const knownSessionIds = useRef(new Set<string>())
  const prevActive = useRef(active)

  // ── 全量加载 ──────────────────────────────────────

  const load = useCallback(async () => {
    dispatch({ type: "SET_LOADING" })
    try {
      const [projectData, annotations] = await Promise.all([
        scanProjects(),
        getAnnotations(),
      ])
      // 初始化已知 session 集合
      const ids = new Set<string>()
      for (const pd of projectData) {
        for (const s of pd.sessions) ids.add(s.id)
      }
      knownSessionIds.current = ids
      dispatch({ type: "SET_DATA", projectData, annotations })
    } catch (e) {
      dispatch({ type: "SET_ERROR", error: String(e) })
    }
  }, [])

  // ── 静默刷新：检测新 session → flash card ─────────

  const backgroundRefresh = useCallback(async () => {
    try {
      const [projectData, annotations] = await Promise.all([
        scanProjects(),
        getAnnotations(),
      ])
      // 检测新 session
      let newest: SessionMeta | null = null
      for (const pd of projectData) {
        for (const s of pd.sessions) {
          if (!knownSessionIds.current.has(s.id)) {
            if (!newest || s.started_at > newest.started_at) newest = s
          }
        }
      }
      // 更新已知集合
      const ids = new Set<string>()
      for (const pd of projectData) {
        for (const s of pd.sessions) ids.add(s.id)
      }
      knownSessionIds.current = ids
      dispatch({ type: "REFRESH_DATA", projectData, annotations })
      if (newest) dispatch({ type: "SET_FLASH", session: newest })
    } catch (_) {
      // 静默失败：后台刷新不打扰用户
    }
  }, [])

  // ── 初始加载 ──────────────────────────────────────

  useEffect(() => { load() }, [load])

  // ── re-entry 检测 ─────────────────────────────────

  useEffect(() => {
    if (active && !prevActive.current) backgroundRefresh()
    prevActive.current = active
  }, [active, backgroundRefresh])

  // ── 派生数据（全部 useMemo） ──────────────────────

  const filteredProjects = useMemo(() => {
    const q = state.search.toLowerCase()
    return state.projectData
      .map(pd => pd.project)
      .filter(p => !q || p.name.toLowerCase().includes(q))
      .sort((a, b) => b.last_active.localeCompare(a.last_active))
  }, [state.projectData, state.search])

  const selectedSessions = useMemo(() => {
    const pd = state.projectData.find(d => d.project.id === state.selectedProjectId)
    if (!pd) return []
    return [...pd.sessions].sort((a, b) => b.started_at.localeCompare(a.started_at))
  }, [state.projectData, state.selectedProjectId])

  const filteredSessions = useMemo(() => {
    if (state.tagFilter.length === 0) return selectedSessions
    const hasUntagged = state.tagFilter.includes("untagged")
    const realTags = state.tagFilter.filter(t => t !== "untagged")
    return selectedSessions.filter(s => {
      const ann = state.annotations[s.id]
      const tags = ann?.tags ?? []
      if (hasUntagged && tags.length === 0) return true
      if (realTags.length > 0 && realTags.some(t => tags.includes(t))) return true
      return false
    })
  }, [selectedSessions, state.tagFilter, state.annotations])

  const allSessions = useMemo(() => {
    return state.projectData.flatMap(pd => pd.sessions)
  }, [state.projectData])

  const morningFocus: MorningFocus = useMemo(() => {
    const yday = yesterdayString()
    const ydaySessions = allSessions.filter(
      s => localDateString(s.started_at) === yday,
    )
    const sessionCount = ydaySessions.length
    const avgTurns = sessionCount > 0
      ? Math.round(ydaySessions.reduce((s, x) => s + x.turn_count, 0) / sessionCount)
      : 0
    let efficient = 0
    let pitfall = 0
    for (const s of ydaySessions) {
      const tags = state.annotations[s.id]?.tags ?? []
      if (tags.includes("efficient")) efficient++
      if (tags.includes("pitfall")) pitfall++
    }
    return { sessionCount, avgTurns, efficient, pitfall }
  }, [allSessions, state.annotations])

  const tagDistribution: DistributionItem[] = useMemo(() => {
    const counts = new Map<string, number>()
    for (const opt of TAG_OPTIONS) counts.set(opt.label, 0)
    for (const s of allSessions) {
      const tags = state.annotations[s.id]?.tags ?? []
      for (const t of tags) {
        const opt = TAG_OPTIONS.find(o => o.id === t)
        if (opt) counts.set(opt.label, (counts.get(opt.label) ?? 0) + 1)
      }
    }
    const items = [...counts.entries()].map(([label, count]) => ({ label, count }))
    return computeDistribution(items)
  }, [allSessions, state.annotations])

  // ── 操作 ──────────────────────────────────────────

  const selectProject = useCallback(
    (id: string | null) => dispatch({ type: "SELECT_PROJECT", id }),
    [],
  )

  const setSearch = useCallback(
    (search: string) => dispatch({ type: "SET_SEARCH", search }),
    [],
  )

  const setTagFilter = useCallback(
    (tags: string[]) => dispatch({ type: "SET_TAG_FILTER", tags }),
    [],
  )

  const dismissFlash = useCallback(
    () => dispatch({ type: "SET_FLASH", session: null }),
    [],
  )

  const annotateSession = useCallback(async (
    sessionId: string,
    tags: string[],
    note: string | null,
  ) => {
    await apiAnnotate(sessionId, tags, note)
    const annotations = await getAnnotations()
    dispatch({ type: "SET_ANNOTATIONS", annotations })
  }, [])

  const removeAnnotation = useCallback(async (sessionId: string) => {
    await apiRemove(sessionId)
    const annotations = await getAnnotations()
    dispatch({ type: "SET_ANNOTATIONS", annotations })
  }, [])

  return {
    // 状态
    loading: state.loading,
    error: state.error,
    annotations: state.annotations,
    selectedProjectId: state.selectedProjectId,
    search: state.search,
    tagFilter: state.tagFilter,
    flashSession: state.flashSession,
    // 派生数据
    filteredProjects,
    selectedSessions,
    filteredSessions,
    allSessions,
    morningFocus,
    tagDistribution,
    // 操作
    selectProject,
    setSearch,
    setTagFilter,
    dismissFlash,
    annotateSession,
    removeAnnotation,
    refresh: load,
    backgroundRefresh,
  }
}
