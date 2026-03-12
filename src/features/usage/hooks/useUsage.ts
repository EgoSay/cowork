/**
 * [INPUT]: 依赖 @/lib/api::getUsageData, @/lib/types, ../lib
 * [OUTPUT]: 对外提供 useUsage hook（单真相源，所有聚合从 DailyRecord[] 派生）
 * [POS]: usage hooks 核心，管理仪表盘状态
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useMemo, useReducer } from "react"
import { getUsageData } from "@/lib/api"
import type { UsageData, Tool } from "@/lib/types"
import { type TimeRange, cutoffDate, recordTotal } from "../lib"

interface State {
  data: UsageData | null
  timeRange: TimeRange
  loading: boolean
  error: string | null
}

type Action =
  | { type: "SET_LOADING" }
  | { type: "SET_DATA"; data: UsageData }
  | { type: "SET_ERROR"; error: string }
  | { type: "SET_RANGE"; range: TimeRange }

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "SET_LOADING":
      return { ...state, loading: true, error: null }
    case "SET_DATA":
      return { ...state, data: action.data, loading: false }
    case "SET_ERROR":
      return { ...state, error: action.error, loading: false }
    case "SET_RANGE":
      return { ...state, timeRange: action.range }
  }
}

// ── 派生类型 ─────────────────────────────────────────────

export interface DailyTotal {
  date: string
  claude: number
  codex: number
}

export interface ModelTotal {
  model: string
  tool: Tool
  input_tokens: number
  output_tokens: number
  cache_read_tokens: number
  cache_write_tokens: number
}

// ── Hook ─────────────────────────────────────────────────

export function useUsage() {
  const [state, dispatch] = useReducer(reducer, {
    data: null,
    timeRange: "week",
    loading: true,
    error: null,
  })

  const load = useCallback(async () => {
    dispatch({ type: "SET_LOADING" })
    try {
      const data = await getUsageData()
      dispatch({ type: "SET_DATA", data })
    } catch (e) {
      dispatch({ type: "SET_ERROR", error: String(e) })
    }
  }, [])

  useEffect(() => { load() }, [load])

  // ── 过滤后的记录 ────────────────────────────────────
  const cutoff = useMemo(() => cutoffDate(state.timeRange), [state.timeRange])
  const filtered = useMemo(() => {
    if (!state.data) return []
    return state.data.records.filter(r => r.date >= cutoff)
  }, [state.data, cutoff])

  // ── 总 token ────────────────────────────────────────
  const totalTokens = useMemo(
    () => filtered.reduce((s, r) => s + recordTotal(r), 0),
    [filtered],
  )

  // ── 日聚合（趋势图） ──────────────────────────────
  const dailyTotals: DailyTotal[] = useMemo(() => {
    const map = new Map<string, DailyTotal>()
    for (const r of filtered) {
      const entry = map.get(r.date) ?? { date: r.date, claude: 0, codex: 0 }
      const total = recordTotal(r)
      if (r.tool === "claude_code") entry.claude += total
      else if (r.tool === "codex") entry.codex += total
      map.set(r.date, entry)
    }
    return [...map.values()].sort((a, b) => a.date.localeCompare(b.date))
  }, [filtered])

  // ── 模型聚合（含 breakdown，单真相源） ────────────
  const modelTotals: ModelTotal[] = useMemo(() => {
    const map = new Map<string, ModelTotal>()
    for (const r of filtered) {
      const key = `${r.tool}:${r.model}`
      const entry = map.get(key) ?? {
        model: r.model, tool: r.tool,
        input_tokens: 0, output_tokens: 0,
        cache_read_tokens: 0, cache_write_tokens: 0,
      }
      entry.input_tokens += r.input_tokens
      entry.output_tokens += r.output_tokens
      entry.cache_read_tokens += r.cache_read_tokens
      entry.cache_write_tokens += r.cache_write_tokens
      map.set(key, entry)
    }
    const total = (m: ModelTotal) =>
      m.input_tokens + m.output_tokens + m.cache_read_tokens + m.cache_write_tokens
    return [...map.values()].sort((a, b) => total(b) - total(a))
  }, [filtered])

  return {
    timeRange: state.timeRange,
    setTimeRange: (r: TimeRange) => dispatch({ type: "SET_RANGE", range: r }),
    loading: state.loading,
    error: state.error,
    refresh: load,
    totalTokens,
    dailyTotals,
    modelTotals,
    scannedUntil: state.data?.scanned_until ?? "",
  }
}
