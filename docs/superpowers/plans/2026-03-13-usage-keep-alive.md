# Usage Keep-Alive + Stale-While-Revalidate Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 消除 Usage 页面每次切入时的 loading 等待，实现瞬显缓存 + 后台静默刷新。

**Architecture:** App.tsx 从条件渲染改为 keep-alive 模式（首次访问后保持挂载，CSS hidden 隐藏）。useUsage hook 新增 backgroundRefresh（跳过 SET_LOADING，静默更新数据）。UsagePage 接收 active prop，re-entry 时自动触发 backgroundRefresh。

**Tech Stack:** React 18 (useReducer, useCallback, useEffect, useRef), Tailwind CSS

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src/features/usage/hooks/useUsage.ts` | Modify | 新增 backgroundRefresh 回调，跳过 loading 态静默拉取数据 |
| `src/features/usage/pages/UsagePage.tsx` | Modify | 接收 active prop，re-entry 时触发 backgroundRefresh |
| `src/App.tsx` | Modify | 条件渲染 → keep-alive 模式（mount-once, CSS hidden） |

---

## Task 1: useUsage — 新增 backgroundRefresh

**Files:**
- Modify: `src/features/usage/hooks/useUsage.ts:75-83` (load 函数附近)
- Modify: `src/features/usage/hooks/useUsage.ts:167-181` (return 对象)

**Context:** 当前 `load()` 函数 dispatch SET_LOADING → loading=true → 显示 "Loading..."。backgroundRefresh 需要跳过 SET_LOADING，直接拉取数据后 dispatch SET_DATA，用户看到的是旧数据无缝替换为新数据。

- [ ] **Step 1: 在 load 函数下方添加 backgroundRefresh**

在 `useUsage.ts` 的 `load` 函数（第 75-83 行）后面，`useEffect` 前面，添加：

```typescript
// ── 静默刷新：不触发 loading 态，旧数据无缝替换 ────
const backgroundRefresh = useCallback(async () => {
  try {
    const data = await getUsageData()
    dispatch({ type: "SET_DATA", data })
  } catch (_) {
    // 静默失败：后台刷新不打扰用户
  }
}, [])
```

- [ ] **Step 2: 在 return 对象中暴露 backgroundRefresh**

将 return 对象修改为：

```typescript
return {
  timeRange: state.timeRange,
  displayFrom: bounds.from,
  displayTo: bounds.to,
  scanWindow,
  setTimeRange,
  setCustomRange,
  loading: state.loading,
  error: state.error,
  refresh: load,
  backgroundRefresh,              // 新增
  totalTokens,
  dailyTotals,
  modelTotals,
  scannedUntil: state.data?.scanned_until ?? "",
}
```

- [ ] **Step 3: 更新 L3 头部注释**

```typescript
/**
 * [INPUT]: 依赖 @/lib/api::getUsageData, @/lib/types, ../lib
 * [OUTPUT]: 对外提供 useUsage hook（单真相源，displayFrom/displayTo 始终反映当前筛选范围，backgroundRefresh 静默刷新）
 * [POS]: usage hooks 核心，管理仪表盘状态
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
```

- [ ] **Step 4: 验证类型检查**

Run: `pnpm typecheck`
Expected: 无错误

- [ ] **Step 5: 提交**

```bash
git add src/features/usage/hooks/useUsage.ts
git commit -m "feat(usage): add backgroundRefresh for stale-while-revalidate"
```

---

## Task 2: UsagePage — 接收 active prop，re-entry 静默刷新

**Files:**
- Modify: `src/features/usage/pages/UsagePage.tsx:1-19` (props + hook 调用区域)

**Context:** UsagePage 当前无 props。新增 `active` prop，当 active 从 false→true 时触发 backgroundRefresh。用 useRef 记录上一次 active 值，避免首次挂载时重复 fetch（load 已经在 useEffect 中触发）。

- [ ] **Step 1: 添加 active prop 和 re-entry effect**

修改 `UsagePage.tsx`，在 import 中加入 `useEffect, useRef`，添加 props 接口和 re-entry 逻辑：

```typescript
import { useEffect, useRef } from "react"
import { TimeRangeTab } from "../components/TimeRangeTab"
import { SummaryCards } from "../components/SummaryCards"
import { DailyChart } from "../components/DailyChart"
import { ModelTable } from "../components/ModelTable"
import { useUsage } from "../hooks/useUsage"

interface UsagePageProps {
  active?: boolean
}

export function UsagePage({ active = true }: UsagePageProps) {
  const {
    timeRange, displayFrom, displayTo, scanWindow,
    setTimeRange, setCustomRange,
    loading, error, refresh, backgroundRefresh,
    totalTokens, dailyTotals, modelTotals, scannedUntil,
  } = useUsage()

  // ── re-entry 静默刷新（跳过首次挂载，只在 false→true 时触发）──
  const prevActive = useRef(active)
  useEffect(() => {
    if (active && !prevActive.current) backgroundRefresh()
    prevActive.current = active
  }, [active, backgroundRefresh])

  // ... 其余 JSX 不变
```

- [ ] **Step 2: 更新 L3 头部注释**

```typescript
/**
 * [INPUT]: 依赖 react, TimeRangeTab, SummaryCards, DailyChart, ModelTable, useUsage
 * [OUTPUT]: 对外提供 UsagePage 组件（支持 active prop，re-entry 静默刷新）
 * [POS]: usage pages 的主仪表盘视图，被 App.tsx 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
```

- [ ] **Step 3: 验证类型检查**

Run: `pnpm typecheck`
Expected: 无错误

- [ ] **Step 4: 提交**

```bash
git add src/features/usage/pages/UsagePage.tsx
git commit -m "feat(usage): add active prop with re-entry background refresh"
```

---

## Task 3: App.tsx — keep-alive 模式

**Files:**
- Modify: `src/App.tsx:31-48` (render 区域)

**Context:** 当前三个模块用 `{activeModule === "xxx" && <Page />}` 条件渲染，切走时组件卸载。改为 mount-once keep-alive：首次访问后保持挂载，用 Tailwind `hidden` class 隐藏。`contents` class 让 wrapper div 透明不影响布局。

策略：用 `visited` Set 跟踪已访问模块，只有访问过的模块才挂载（避免一开始挂载所有模块浪费资源）。

- [ ] **Step 1: 改造 App.tsx render 区域**

```typescript
function App() {
  const [activeModule, setActiveModule] = useState("skills")
  const [view, setView] = useState<View>({ kind: "list" })
  const [visited, setVisited] = useState(() => new Set(["skills"]))

  // ── 模块切换时标记已访问 ──
  const handleModuleChange = useCallback((m: string) => {
    setActiveModule(m)
    setVisited(prev => prev.has(m) ? prev : new Set(prev).add(m))
  }, [])

  const handleSelectSkill = useCallback((skill: SkillMeta) => {
    setView({ kind: "detail", skill })
  }, [])

  const handleBack = useCallback(() => {
    setView({ kind: "list" })
  }, [])

  return (
    <AppShell activeModule={activeModule} onModuleChange={handleModuleChange}>
      {/* Skills: 始终挂载（默认模块） */}
      <div className={activeModule === "skills" ? "contents" : "hidden"}>
        {view.kind === "list" && (
          <SkillsPage onSelectSkill={handleSelectSkill} />
        )}
        {view.kind === "detail" && (
          <SkillDetailPage
            skill={view.skill}
            onBack={handleBack}
          />
        )}
      </div>

      {/* Usage: 首次访问后保持挂载 */}
      {visited.has("usage") && (
        <div className={activeModule === "usage" ? "contents" : "hidden"}>
          <UsagePage active={activeModule === "usage"} />
        </div>
      )}

      {/* Config: 首次访问后保持挂载 */}
      {visited.has("config") && (
        <div className={activeModule === "config" ? "contents" : "hidden"}>
          <ProvidersPage />
        </div>
      )}
    </AppShell>
  )
}
```

- [ ] **Step 2: 更新 L3 头部注释**

```typescript
/**
 * [INPUT]: 依赖 @/components/layout/AppShell, @/features/skills 页面, @/features/usage 页面, @/features/providers 页面, @/lib/types
 * [OUTPUT]: 对外提供 App 根组件（keep-alive 模块切换，visited 懒挂载）
 * [POS]: 应用根，管理模块路由和视图状态
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
```

- [ ] **Step 3: 验证类型检查**

Run: `pnpm typecheck`
Expected: 无错误

- [ ] **Step 4: 验证 Rust 测试**

Run: `cd src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test`
Expected: 73 passed

- [ ] **Step 5: 提交**

```bash
git add src/App.tsx
git commit -m "feat: keep-alive module switching with visited lazy mount"
```

---

## Task 4: GEB L2 文档同步

**Files:**
- Modify: `src/features/usage/CLAUDE.md`

- [ ] **Step 1: 更新 L2 成员清单**

在 useUsage.ts 条目中加入 backgroundRefresh 说明：

```markdown
- `hooks/useUsage.ts`: single truth source, displayFrom/displayTo 始终反映当前筛选范围, backgroundRefresh 静默刷新, effectiveCustom memo (auto-clamp on refresh)
```

在 UsagePage.tsx 条目中加入 active prop 说明：

```markdown
- `pages/UsagePage.tsx`: main dashboard (active prop, re-entry backgroundRefresh, summary cards, daily chart, model table)
```

- [ ] **Step 2: 提交**

```bash
git add src/features/usage/CLAUDE.md
git commit -m "docs: update L2 for keep-alive and backgroundRefresh"
```
