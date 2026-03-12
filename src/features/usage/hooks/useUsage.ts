/**
 * [INPUT]: 依赖 @/lib/api::getUsageData, @/lib/types, ../lib
 * [OUTPUT]: 对外提供 useUsage hook（单真相源，所有聚合从 DailyRecord[] 派生）
 * [POS]: usage hooks 核心，管理仪表盘状态
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useMemo, useReducer } from "react"
import { getUsageData } from "@/lib/api"
import type { UsageData, Tool } from "@/lib/types"
import {
  type PresetRange, type TimeRange, type ScanWindow,
  dateRange, clampToWindow, localDateString, recordTotal,
} from "../lib"

interface State {
  data: UsageData | null
  timeRange: TimeRange
  customFrom: string   // "" = sentinel (never explicitly set); raw user intent
  customTo: string     // "" = sentinel; raw user intent
  loading: boolean
  error: string | null
}

type Action =
  | { type: "SET_LOADING" }
  | { type: "SET_DATA"; data: UsageData }
  | { type: "SET_ERROR"; error: string }
  | { type: "SET_RANGE"; range: TimeRange }
  | { type: "SET_CUSTOM"; from: string; to: string }

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
    case "SET_CUSTOM":
      return { ...state, timeRange: "custom", customFrom: action.from, customTo: action.to }
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
    customFrom: "",
    customTo: "",
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

  // ── 扫描窗口：从后端返回值推导 ──────────────────────
  const scanWindow: ScanWindow = useMemo(() => {
    if (!state.data) {
      const today = localDateString()
      return { min: today, max: today }
    }
    return { min: state.data.scanned_from, max: state.data.scanned_until }
  }, [state.data])

  // ── effectiveCustom：state = 用户意图，derived = clamped 现实 ──
  // scanWindow 变化（refresh 后）→ 自动 recompute → 无需副作用
  // write-time clamp (switchToCustom/setCustomRange) 是 belt；
  // derive-time clamp 是 suspenders，专治 refresh 后窗口滑动
  const effectiveCustom = useMemo(() => {
    if (state.customFrom === "") return { from: "", to: "" }
    return clampToWindow(state.customFrom, state.customTo, scanWindow)
  }, [state.customFrom, state.customTo, scanWindow])

  // ── 统一双边界过滤 ─────────────────────────────────
  const bounds = useMemo(
    () => dateRange(state.timeRange, effectiveCustom.from, effectiveCustom.to),
    [state.timeRange, effectiveCustom.from, effectiveCustom.to],
  )
  const filtered = useMemo(() => {
    if (!state.data) return []
    return state.data.records.filter(r => r.date >= bounds.from && r.date <= bounds.to)
  }, [state.data, bounds])

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

  // ── setTimeRange：只接受 PresetRange ─────────────────
  const setTimeRange = useCallback(
    (r: PresetRange) => dispatch({ type: "SET_RANGE", range: r }),
    [],
  )

  // ── Custom 切换：首次继承 preset，之后恢复 ─────────
  // data=null 时 scanWindow 是假值，早退防止错误初始化
  const switchToCustom = useCallback(() => {
    if (!state.data) return
    if (state.customFrom === "") {
      const current = dateRange(state.timeRange)
      const clamped = clampToWindow(current.from, current.to, scanWindow)
      dispatch({ type: "SET_CUSTOM", from: clamped.from, to: clamped.to })
    } else {
      dispatch({ type: "SET_RANGE", range: "custom" })
    }
  }, [state.data, state.timeRange, state.customFrom, scanWindow])

  // ── 日期输入变更 ──────────────────────────────────
  // data=null 时同理早退
  const setCustomRange = useCallback((from: string, to: string) => {
    if (!state.data) return
    const clamped = clampToWindow(from, to, scanWindow)
    dispatch({ type: "SET_CUSTOM", from: clamped.from, to: clamped.to })
  }, [state.data, scanWindow])

  return {
    timeRange: state.timeRange,
    customFrom: effectiveCustom.from,   // 暴露 clamped 版本
    customTo: effectiveCustom.to,       // 暴露 clamped 版本
    scanWindow,
    setTimeRange,
    setCustomRange,
    switchToCustom,
    loading: state.loading,
    error: state.error,
    refresh: load,
    totalTokens,
    dailyTotals,
    modelTotals,
    scannedUntil: state.data?.scanned_until ?? "",
  }
}
