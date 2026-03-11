/**
 * [INPUT]: 依赖 @/lib/api 的 provider 函数, @/lib/types
 * [OUTPUT]: 对外提供 useProviders hook
 * [POS]: providers 的状态管理，被 ProvidersPage 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useReducer } from "react"
import {
  getProviders,
  switchProvider,
  addProvider as apiAddProvider,
  updateProvider as apiUpdateProvider,
  removeProvider as apiRemoveProvider,
} from "@/lib/api"
import type { ProvidersConfig } from "@/lib/types"

interface State {
  config: ProvidersConfig | null
  loading: boolean
  error: string | null
  switching: string | null
}

type Action =
  | { type: "LOAD_START" }
  | { type: "LOAD_OK"; config: ProvidersConfig }
  | { type: "LOAD_ERR"; error: string }
  | { type: "SWITCH_START"; id: string }
  | { type: "SWITCH_OK" }
  | { type: "SWITCH_ERR"; error: string }

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "LOAD_START":
      return { ...state, loading: true, error: null }
    case "LOAD_OK":
      return { ...state, loading: false, config: action.config }
    case "LOAD_ERR":
      return { ...state, loading: false, error: action.error }
    case "SWITCH_START":
      return { ...state, switching: action.id, error: null }
    case "SWITCH_OK":
      return { ...state, switching: null }
    case "SWITCH_ERR":
      return { ...state, switching: null, error: action.error }
  }
}

export function useProviders(toolKey: string = "claude_code") {
  const [state, dispatch] = useReducer(reducer, {
    config: null,
    loading: true,
    error: null,
    switching: null,
  })

  const load = useCallback(async () => {
    dispatch({ type: "LOAD_START" })
    try {
      const config = await getProviders()
      dispatch({ type: "LOAD_OK", config })
    } catch (e) {
      dispatch({ type: "LOAD_ERR", error: String(e) })
    }
  }, [])

  useEffect(() => { load() }, [load])

  const doSwitch = useCallback(async (providerId: string) => {
    dispatch({ type: "SWITCH_START", id: providerId })
    try {
      await switchProvider(toolKey, providerId)
      dispatch({ type: "SWITCH_OK" })
      await load()
    } catch (e) {
      dispatch({ type: "SWITCH_ERR", error: String(e) })
    }
  }, [toolKey, load])

  const doAdd = useCallback(async (
    id: string, name: string, baseUrl: string, apiKey: string
  ) => {
    try {
      await apiAddProvider(id, name, toolKey, baseUrl, apiKey)
      await load()
    } catch (e) {
      dispatch({ type: "LOAD_ERR", error: String(e) })
    }
  }, [toolKey, load])

  const doUpdate = useCallback(async (
    id: string, name?: string, baseUrl?: string, apiKey?: string
  ) => {
    try {
      await apiUpdateProvider(id, name, baseUrl, apiKey)
      await load()
    } catch (e) {
      dispatch({ type: "LOAD_ERR", error: String(e) })
    }
  }, [load])

  const doRemove = useCallback(async (id: string) => {
    try {
      await apiRemoveProvider(id)
      await load()
    } catch (e) {
      dispatch({ type: "LOAD_ERR", error: String(e) })
    }
  }, [load])

  const providers = state.config?.providers.filter(p => p.tool === toolKey) ?? []
  const activeId = state.config?.active[toolKey] ?? ""

  return {
    providers,
    activeId,
    loading: state.loading,
    switching: state.switching,
    error: state.error,
    switchProvider: doSwitch,
    addProvider: doAdd,
    updateProvider: doUpdate,
    removeProvider: doRemove,
    reload: load,
  }
}
