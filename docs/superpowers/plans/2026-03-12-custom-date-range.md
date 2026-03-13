# Custom Date Range Filter Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to select a custom start/end date for token usage filtering, in addition to the existing Today/7D/30D presets.

**Architecture:** Unify filtering to double-bound `{ from, to }`. Scan window = backend `scanned_from`..`scanned_until` (earliest fully-scanned day..today). Backend clips events outside window (`records.retain`), guaranteeing UI/data 同构. Frontend: `PresetRange` and `"custom"` are disjoint entry points — `setTimeRange` only accepts presets, custom mode only via `switchToCustom`. State stores user intent (raw), `effectiveCustom` memo = `clampToWindow(raw, scanWindow)` — auto-recomputes on refresh, zero side effects.

**Tech Stack:** Rust + Tauri 2 (backend), React 18 + TypeScript + Tailwind v4 (frontend)

---

## Design Decisions

1. **Scan window = 已扫描的完整日期范围.** Backend mtime cutoff = now - 31*86400s. Day `now-31d` is partially scanned. `scanned_from = now - 30d` (first day where all 24h fall within mtime window). `scanned_until = today`. Backend clips events outside this window.

2. **`records.retain()` in `parse_all()`** clips events outside `[scanned_from, scanned_until]`. No dead data reaches frontend. Testable via extracted `scan_window_dates()` helper.

3. **`PresetRange` vs `"custom"` are disjoint entry points.** `PresetRange = "today" | "week" | "month"`. `TimeRange = PresetRange | "custom"`. Public `setTimeRange` only accepts `PresetRange`. Custom mode only via `switchToCustom()`. Prevents bypassing the sentinel/clamp invariant.

4. **State = user intent, derived = clamped reality.** `effectiveCustom = useMemo(clampToWindow(raw, scanWindow))`. When `scanWindow` shifts after refresh, memo auto-recomputes. Zero side effects.

5. **`clampToWindow(from, to, window)`** is the single normalization point. Handles: empty → window.max, swap, out-of-window → 双向 clamp (floor + ceiling), post-clamp re-inversion → collapse。双向 clamp 保证结果永远在 [win.min, win.max] 内。

6. **Custom click behavior**: `""` sentinel → inherit preset (clamped). Non-empty → restore (derive-time clamp handles window shift).

7. **Loading 期间双层保护.** `scanWindow` 在 data=null 时是假值 (today/today)。两层防线：(1) hook 层：`switchToCustom()` / `setCustomRange()` 在 `!state.data` 时早退，不写假值到 raw state；(2) UI 层：`TimeRangeTab` 接受 `disabled` prop，`loading` 时整体不可交互。Belt + suspenders。

8. **`LOOKBACK_DAYS` 单一真相源.** 常量定义在 `parser/mod.rs`，子模块 `claude_code.rs` 和 `codex.rs` 通过 `super::LOOKBACK_DAYS` 引用。消除三处独立魔数，任何窗口变更只需改一处。

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `src-tauri/src/features/usage/types.rs` | Add `scanned_from` to UsageData |
| Modify | `src-tauri/src/features/usage/parser/mod.rs` | `LOOKBACK_DAYS` 单一真相源, `scan_window_dates()` helper, clip events, 2 tests |
| Modify | `src-tauri/src/features/usage/parser/claude_code.rs` | 移除本地 LOOKBACK_DAYS, 改用 `super::LOOKBACK_DAYS` |
| Modify | `src-tauri/src/features/usage/parser/codex.rs` | 移除本地 LOOKBACK_SECS, 改用 `super::LOOKBACK_DAYS` |
| Modify | `src-tauri/src/features/usage/parser/CLAUDE.md` | Update L2 (scan window + clip 职责 + 单一常量) |
| Modify | `src/lib/types.ts` | Add `scanned_from` to TS UsageData |
| Modify | `src/features/usage/lib.ts` | `PresetRange`, `"custom"`, `dateRange`, `clampToWindow`, `ScanWindow` |
| Modify | `src/features/usage/hooks/useUsage.ts` | `effectiveCustom` memo, `setTimeRange(PresetRange)`, `switchToCustom` |
| Modify | `src/features/usage/components/TimeRangeTab.tsx` | Custom pill + date inputs, `onChange(PresetRange)`, `disabled` guard |
| Modify | `src/features/usage/pages/UsagePage.tsx` | Wire new props (含 loading → disabled) |
| Modify | `src/features/usage/CLAUDE.md` | Update L2 |
| Modify | `src-tauri/src/features/usage/CLAUDE.md` | Update L2 |

**Total: 12 files modified, 0 files created**

---

## Chunk 1: Implementation

### Task 1: Backend — scanned_from + event clipping + tests

**Files:**
- Modify: `src-tauri/src/features/usage/types.rs:22-27`
- Modify: `src-tauri/src/features/usage/parser/mod.rs` (full rewrite)
- Modify: `src-tauri/src/features/usage/parser/claude_code.rs:15,133`
- Modify: `src-tauri/src/features/usage/parser/codex.rs:55,136`
- Modify: `src-tauri/src/features/usage/parser/CLAUDE.md`
- Modify: `src/lib/types.ts:62-65`

- [ ] **Step 1: Add `scanned_from` to Rust UsageData**

In `src-tauri/src/features/usage/types.rs`, replace lines 22-27:

```rust
// ── 完整响应 ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageData {
    pub records: Vec<DailyRecord>,
    pub scanned_from: String,    // 最早完整可选日 (= now - 30d, 本地时区)
    pub scanned_until: String,   // 扫描截止日期 (= today, 本地时区)
}
```

Update L3 OUTPUT:
```
 * [OUTPUT]: 对外提供 DailyRecord, UsageData (含 scanned_from/scanned_until 扫描窗口)
```

- [ ] **Step 2: Replace full `parser/mod.rs` with LOOKBACK_DAYS 单一真相源 + scan_window_dates + clip + tests**

```rust
/**
 * [INPUT]: 依赖 claude_code, codex 子模块, chrono (含 Duration), super::types
 * [OUTPUT]: 对外提供 LOOKBACK_DAYS, parse_all(), scan_window_dates(), timestamp_to_date()
 * [POS]: parser/ 入口，定义扫描窗口常量（单一真相源），协调解析，裁剪窗口外事件
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
mod claude_code;
mod codex;

use super::types::{DailyRecord, UsageData};
use chrono::{DateTime, Duration, Local, TimeZone};

// ── 扫描窗口常量（单一真相源）────────────────────────────
// 子模块 claude_code / codex 通过 super::LOOKBACK_DAYS 引用
pub(crate) const LOOKBACK_DAYS: u64 = 31;

// ── 共享：时间戳 → 本地日期 ────────────────────────────

pub(crate) fn timestamp_to_date<Tz: TimeZone>(ts: &str, tz: &Tz) -> Option<String>
where
    Tz::Offset: std::fmt::Display,
{
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        return Some(dt.with_timezone(tz).format("%Y-%m-%d").to_string());
    }
    if ts.len() >= 10 && ts.as_bytes()[4] == b'-' && ts.as_bytes()[7] == b'-' {
        return Some(ts[..10].to_string());
    }
    None
}

// ── 扫描窗口日期：最早完整可选日 .. 今天 ────────────────
// mtime cutoff 是 now-31d（秒级），所以 now-31d 那天只被部分扫描
// now-30d 是第一个 24h 完整在窗口内的日期

pub(crate) fn scan_window_dates<Tz: TimeZone>(now: &DateTime<Tz>) -> (String, String)
where
    Tz::Offset: std::fmt::Display,
{
    // DateTime<Tz> 在 chrono 0.4 是 Clone 不是 Copy，不能 *now
    let from = (now.clone() - Duration::days(LOOKBACK_DAYS as i64 - 1))
        .format("%Y-%m-%d").to_string();
    let until = now.format("%Y-%m-%d").to_string();
    (from, until)
}

pub fn parse_all() -> UsageData {
    let now = Local::now();
    let (scanned_from, scanned_until) = scan_window_dates(&now);

    let mut records = claude_code::parse();
    records.extend(codex::parse());
    // SAFETY: retain 必须保留 — 由 retain_clips_events_outside_window 测试锁定
    records.retain(|r| r.date >= scanned_from && r.date <= scanned_until);

    UsageData { records, scanned_from, scanned_until }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Tool;
    use chrono::FixedOffset;

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(8 * 3600).unwrap()
    }

    #[test]
    fn scan_window_is_30_day_range() {
        let tz = tz();
        let now = tz.with_ymd_and_hms(2026, 3, 12, 14, 0, 0).unwrap();
        let (from, until) = scan_window_dates(&now);
        assert_eq!(from, "2026-02-10");  // 30 days before (first fully scanned)
        assert_eq!(until, "2026-03-12");
    }

    #[test]
    fn retain_clips_events_outside_window() {
        fn rec(date: &str) -> DailyRecord {
            DailyRecord {
                date: date.into(), tool: Tool::ClaudeCode, model: "m".into(),
                input_tokens: 1, output_tokens: 0,
                cache_read_tokens: 0, cache_write_tokens: 0,
            }
        }
        let mut records = vec![
            rec("2026-02-09"),  // before window → clipped
            rec("2026-02-10"),  // boundary (in)
            rec("2026-03-05"),  // middle (in)
            rec("2026-03-12"),  // boundary (in)
            rec("2026-03-13"),  // after window → clipped
        ];
        let from = "2026-02-10".to_string();
        let until = "2026-03-12".to_string();
        records.retain(|r| r.date >= from && r.date <= until);
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].date, "2026-02-10");
        assert_eq!(records[1].date, "2026-03-05");
        assert_eq!(records[2].date, "2026-03-12");
    }
}
```

- [ ] **Step 3: Migrate claude_code.rs to use `super::LOOKBACK_DAYS`**

In `src-tauri/src/features/usage/parser/claude_code.rs`:

Remove line 15:
```rust
// DELETE: const LOOKBACK_DAYS: u64 = 31;
```

Replace line 133 (`LOOKBACK_DAYS * 86400`):
```rust
        - std::time::Duration::from_secs(super::LOOKBACK_DAYS * 86_400);
```

- [ ] **Step 4: Migrate codex.rs to use `super::LOOKBACK_DAYS`**

In `src-tauri/src/features/usage/parser/codex.rs`:

Remove line 55:
```rust
// DELETE: const LOOKBACK_SECS: u64 = 31 * 86400;
```

Replace line 136 (`LOOKBACK_SECS`):
```rust
        - std::time::Duration::from_secs(super::LOOKBACK_DAYS * 86_400);
```

- [ ] **Step 5: Add `scanned_from` to TS UsageData**

In `src/lib/types.ts`, replace lines 62-65:

```typescript
export interface UsageData {
  records: DailyRecord[]
  scanned_from: string
  scanned_until: string
}
```

- [ ] **Step 6: Update parser/CLAUDE.md**

```markdown
# features/usage/parser/
> L2 | Parent: src-tauri/src/features/usage/

Session JSONL parsers with unified token accounting. LOOKBACK_DAYS 定义在 mod.rs（单一真相源）。

## Members
- `mod.rs`: LOOKBACK_DAYS 常量（单一真相源）, parse_all() coordinator — defines scan window via scan_window_dates(), merges Claude + Codex records, clips events outside [scanned_from, scanned_until]; shared timestamp_to_date
- `claude_code.rs`: scans ~/.claude/projects/**/*.jsonl (含 subagents, mtime via super::LOOKBACK_DAYS), dedup by message.id, sums per (date, model)
- `codex.rs`: glob+mtime 扫描 ~/.codex/sessions/**/*.jsonl (mtime via super::LOOKBACK_DAYS), incremental last_token_usage + event timestamp 做日归属

## Token Accounting
Claude: 4 independent fields from API → direct mapping
Codex: cached_input ⊂ input → subtract to normalize: input = api.input - api.cached

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
```

- [ ] **Step 7: Verify**

```bash
pnpm typecheck
(cd src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test)
```

Expected: TypeScript clean. All Rust tests pass (now 58: 56 existing + 2 new).

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/features/usage/types.rs src-tauri/src/features/usage/parser/mod.rs src-tauri/src/features/usage/parser/claude_code.rs src-tauri/src/features/usage/parser/codex.rs src-tauri/src/features/usage/parser/CLAUDE.md src/lib/types.ts
git commit -m "feat(usage): LOOKBACK_DAYS single source, scanned_from, scan_window_dates, clip events"
```

---

### Task 2: lib.ts — PresetRange + dateRange + clampToWindow

**Files:**
- Modify: `src/features/usage/lib.ts`

- [ ] **Step 1: Replace full content**

```typescript
/**
 * [INPUT]: 依赖 @/lib/types::DailyRecord
 * [OUTPUT]: 对外提供 PresetRange, TimeRange, DateRange, ScanWindow, formatTokens, localDateString, dateRange, clampToWindow, recordTotal
 * [POS]: usage 模块共享工具，被 hooks 和 components 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { DailyRecord } from "@/lib/types"

// ── 时间范围类型 ─────────────────────────────────────────
// PresetRange 和 "custom" 是不同的入口：
// preset 通过 setTimeRange(PresetRange) 进入
// custom 只通过 switchToCustom() 进入，不可通过 setTimeRange 设置

export type PresetRange = "today" | "week" | "month"
export type TimeRange = PresetRange | "custom"

export interface DateRange {
  from: string
  to: string
}

export interface ScanWindow {
  min: string
  max: string
}

export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return String(n)
}

// ── 本地时区日期字符串 (绝不用 toISOString) ──────────────

export function localDateString(d: Date = new Date()): string {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, "0")
  const day = String(d.getDate()).padStart(2, "0")
  return `${y}-${m}-${day}`
}

// ── 统一双边界：preset 和 custom 走同一路径 ──────────────
// defense in depth: empty → today, from > to → swap
// 主要归一化在 clampToWindow()，这里是安全网

export function dateRange(
  range: TimeRange,
  customFrom?: string,
  customTo?: string,
): DateRange {
  const today = localDateString()
  switch (range) {
    case "today":
      return { from: today, to: today }
    case "week": {
      const d = new Date()
      d.setDate(d.getDate() - 6)
      return { from: localDateString(d), to: today }
    }
    case "month": {
      const d = new Date()
      d.setDate(d.getDate() - 29)
      return { from: localDateString(d), to: today }
    }
    case "custom": {
      // safety net only — callers should go through clampToWindow()
      let f = customFrom || today
      let t = customTo || today
      if (f > t) [f, t] = [t, f]
      return { from: f, to: t }
    }
  }
}

// ── 扫描窗口 clamp：所有写入 customFrom/To 的路径必经 ────
// 处理：空值 → window.max, 倒置 → swap, 越界 → 双向 clamp

export function clampToWindow(
  from: string,
  to: string,
  win: ScanWindow,
): DateRange {
  let f = from || win.max
  let t = to || win.max
  if (f > t) [f, t] = [t, f]
  if (f < win.min) f = win.min
  if (t < win.min) t = win.min     // 两端都早于窗口 → 坍缩到 win.min
  if (t > win.max) t = win.max
  if (f > t) f = t                 // window 可能是单点，clamp 后可能再次倒置
  return { from: f, to: t }
}

// ── DailyRecord 总 token（统一口径） ────────────────────

export function recordTotal(r: DailyRecord): number {
  return r.input_tokens + r.output_tokens + r.cache_read_tokens + r.cache_write_tokens
}
```

Key addition from v4: `PresetRange` type, with comment explaining the invariant.

- [ ] **Step 2: Verify typecheck (expect errors in useUsage.ts — cutoffDate removed)**

Run: `pnpm typecheck 2>&1 | head -20`

- [ ] **Step 3: Commit**

```bash
git add src/features/usage/lib.ts
git commit -m "refactor(usage): PresetRange type, dateRange + clampToWindow"
```

---

### Task 3: useUsage hook — effectiveCustom, setTimeRange(PresetRange)

**Files:**
- Modify: `src/features/usage/hooks/useUsage.ts`

- [ ] **Step 1: Replace full hook**

```typescript
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
    // setTimeRange 只接受 PresetRange，custom 模式只能通过 switchToCustom 进入
    setTimeRange: (r: PresetRange) => dispatch({ type: "SET_RANGE", range: r }),
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
```

Key differences from v4:
- `setTimeRange` takes `PresetRange` (not `TimeRange`) — `"custom"` is impossible to pass (Issue #1)
- `SET_RANGE` action internally still accepts `TimeRange` — used by `switchToCustom` restore branch
- Import adds `PresetRange`
- Comment on `effectiveCustom` explains when derive-time clamp fires

- [ ] **Step 2: Verify typecheck**

Run: `pnpm typecheck 2>&1 | head -20`

Expected: Errors in `TimeRangeTab.tsx` (new props). Hook type-clean.

- [ ] **Step 3: Commit**

```bash
git add src/features/usage/hooks/useUsage.ts
git commit -m "feat(usage): effectiveCustom memo, setTimeRange(PresetRange) type safety"
```

---

### Task 4: TimeRangeTab + UsagePage + docs + acceptance

**Files:**
- Modify: `src/features/usage/components/TimeRangeTab.tsx`
- Modify: `src/features/usage/pages/UsagePage.tsx`
- Modify: `src/features/usage/CLAUDE.md`
- Modify: `src-tauri/src/features/usage/CLAUDE.md`

- [ ] **Step 1: Replace TimeRangeTab**

```typescript
/**
 * [INPUT]: 依赖 ../lib::PresetRange, ../lib::TimeRange, ../lib::ScanWindow
 * [OUTPUT]: 对外提供 TimeRangeTab 组件（含 Custom 日期选择器，扫描窗口边界，loading 禁用）
 * [POS]: usage components 的时间范围选择器
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { PresetRange, TimeRange, ScanWindow } from "../lib"

interface TimeRangeTabProps {
  active: TimeRange
  customFrom: string
  customTo: string
  scanWindow: ScanWindow
  disabled?: boolean                          // loading 时禁用，防止假 scanWindow 初始化 custom
  onChange: (range: PresetRange) => void       // 只接受 preset
  onCustomChange: (from: string, to: string) => void
  onSwitchCustom: () => void                   // custom 的唯一入口
}

const presets: { id: PresetRange; label: string }[] = [
  { id: "today", label: "Today" },
  { id: "week", label: "7 Days" },
  { id: "month", label: "30 Days" },
]

export function TimeRangeTab({
  active, customFrom, customTo, scanWindow, disabled,
  onChange, onCustomChange, onSwitchCustom,
}: TimeRangeTabProps) {
  return (
    <div className={`flex items-center gap-1 ${disabled ? "opacity-50 pointer-events-none" : ""}`}>
      {presets.map((r) => (
        <button
          key={r.id}
          disabled={disabled}
          onClick={() => onChange(r.id)}
          className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
            active === r.id
              ? "bg-text text-bg"
              : "text-text-muted hover:text-text-secondary"
          }`}
        >
          {r.label}
        </button>
      ))}

      {/* Custom pill: 走 switchToCustom (首次继承 preset / 之后恢复) */}
      <button
        disabled={disabled}
        onClick={onSwitchCustom}
        className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
          active === "custom"
            ? "bg-text text-bg"
            : "text-text-muted hover:text-text-secondary"
        }`}
      >
        Custom
      </button>

      {/* 日期选择器：仅在 custom 模式下显示，边界 = 扫描窗口 */}
      {active === "custom" && (
        <div className="flex items-center gap-1 ml-2">
          <input
            type="date"
            disabled={disabled}
            value={customFrom}
            min={scanWindow.min}
            max={customTo}
            onChange={(e) => onCustomChange(e.target.value, customTo)}
            className="px-1.5 py-0.5 rounded bg-bg-card border border-border text-xs text-text"
          />
          <span className="text-text-muted text-xs">–</span>
          <input
            type="date"
            disabled={disabled}
            value={customTo}
            min={customFrom}
            max={scanWindow.max}
            onChange={(e) => onCustomChange(customFrom, e.target.value)}
            className="px-1.5 py-0.5 rounded bg-bg-card border border-border text-xs text-text"
          />
        </div>
      )}
    </div>
  )
}
```

Key: `onChange: (range: PresetRange) => void` — TypeScript prevents passing `"custom"`. `disabled` 防止 loading 期间用假 scanWindow 初始化 custom。

- [ ] **Step 2: Replace UsagePage**

```typescript
/**
 * [INPUT]: 依赖 TimeRangeTab, SummaryCards, DailyChart, ModelTable, useUsage
 * [OUTPUT]: 对外提供 UsagePage 组件
 * [POS]: usage pages 的主仪表盘视图，被 App.tsx 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { TimeRangeTab } from "../components/TimeRangeTab"
import { SummaryCards } from "../components/SummaryCards"
import { DailyChart } from "../components/DailyChart"
import { ModelTable } from "../components/ModelTable"
import { useUsage } from "../hooks/useUsage"

export function UsagePage() {
  const {
    timeRange, customFrom, customTo, scanWindow,
    setTimeRange, setCustomRange, switchToCustom,
    loading, error, refresh,
    totalTokens, dailyTotals, modelTotals, scannedUntil,
  } = useUsage()

  return (
    <div className="flex flex-col h-full">
      {/* 顶部工具栏 */}
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-border">
        <TimeRangeTab
          active={timeRange}
          customFrom={customFrom}
          customTo={customTo}
          scanWindow={scanWindow}
          disabled={loading}
          onChange={setTimeRange}
          onCustomChange={setCustomRange}
          onSwitchCustom={switchToCustom}
        />
        <button
          onClick={refresh}
          disabled={loading}
          className="px-3 py-1.5 rounded-md text-xs text-text-secondary hover:text-text transition-colors disabled:opacity-50"
        >
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-auto p-4 space-y-6">
        {error && (
          <div className="text-danger text-xs">Error: {error}</div>
        )}

        <SummaryCards total={totalTokens} modelTotals={modelTotals} />

        <div>
          <h3 className="text-xs font-medium text-text-secondary mb-3">Daily Usage</h3>
          {dailyTotals.length > 0 ? (
            <DailyChart data={dailyTotals} />
          ) : (
            <div className="text-text-muted text-xs py-8 text-center">
              No data for this period
            </div>
          )}
        </div>

        <div>
          <h3 className="text-xs font-medium text-text-secondary mb-3">Model Distribution</h3>
          {modelTotals.length > 0 ? (
            <ModelTable data={modelTotals} total={totalTokens} />
          ) : (
            <div className="text-text-muted text-xs py-4 text-center">No data</div>
          )}
        </div>
      </div>

      {/* 底部状态栏 */}
      <div className="px-4 py-2 border-t border-border text-[11px] text-text-muted">
        Data scanned until {scannedUntil} &middot; {totalTokens.toLocaleString()} tokens
      </div>
    </div>
  )
}
```

- [ ] **Step 3: Verify typecheck is clean**

Run: `pnpm typecheck`

Expected: Clean.

- [ ] **Step 4: Update L2 docs**

`src/features/usage/CLAUDE.md`:

```markdown
# features/usage/
> L2 | Parent: src/features/

Token usage monitoring dashboard. Unified 4-field accounting (input/output/cache_read/cache_write).

## Members
- `lib.ts`: PresetRange, TimeRange (含 custom), DateRange, ScanWindow, formatTokens, localDateString, dateRange, clampToWindow, recordTotal
- `hooks/useUsage.ts`: single truth source, effectiveCustom memo (auto-clamp on refresh), setTimeRange(PresetRange), switchToCustom (首次继承/之后恢复)
- `pages/UsagePage.tsx`: main dashboard (summary cards, daily chart, model table with breakdown)
- `components/TimeRangeTab.tsx`: Today/7D/30D/Custom pill selector + date picker (扫描窗口边界, onChange(PresetRange), loading 时 disabled)
- `components/SummaryCards.tsx`: 4-card grid (Total, Sent, Received, Cache Hit)
- `components/DailyChart.tsx`: CSS horizontal bar chart (Claude=text/80, Codex=text/30)
- `components/ModelTable.tsx`: model distribution table with input/output/cache columns

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
```

`src-tauri/src/features/usage/CLAUDE.md`:

```markdown
# features/usage/
> L2 | Parent: src-tauri/src/features/

Token usage data aggregation. Both parsers output unified DailyRecord (4-field breakdown).

## Members
- `mod.rs`: module entry
- `types.rs`: DailyRecord (统一口径: input/output/cache_read/cache_write), UsageData (含 scanned_from/scanned_until 扫描窗口)
- `commands.rs`: get_usage_data Tauri IPC command (spawn_blocking)
- `parser/`: dual-tool log parser, LOOKBACK_DAYS 单一真相源, parse_all() defines scan window + clips events (see parser/CLAUDE.md)

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
```

- [ ] **Step 5: Run full verification**

```bash
pnpm typecheck
cd src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test
```

Expected: TypeScript clean, all Rust tests pass (58 tests).

- [ ] **Step 6: Manual acceptance matrix**

Run `pnpm tauri dev` and verify each scenario:

| # | Scenario | Steps | Expected |
|---|----------|-------|----------|
| A1 | Presets work | Click Today → 7D → 30D | Charts update; pills highlight; no regression |
| A2 | First Custom from 7D | View "7 Days", click Custom | Picker: from = max(6d ago, scanWindow.min), to = scanWindow.max. Data unchanged. |
| A3 | First Custom from 30D | View "30 Days", click Custom | Picker: from = 29d ago (30 Days preset 语义，在 scanWindow 范围内无需 clamp), to = today. |
| A4 | First Custom from Today | View "Today", click Custom | Picker: from = today, to = today. |
| A5 | Adjust custom from | In Custom, change "from" 3 days back | Chart narrows. All aggregations update. |
| A6 | Restore last custom | Custom(3d), click 7D, click Custom | Restores 3-day range, not re-inheriting 7D. |
| A7 | Picker bounds = scan window | Open date picker | from.min = scanned_from (30d ago). to.max = scanned_until (today). |
| A8 | Zero-usage day selectable | Select today if no usage today | Selectable. Shows "No data for this period". |
| A9 | Preset after Custom | Click Today while in Custom | Pill switches; picker hides; data matches. |
| A10 | **Refresh auto-clamps** | Custom(from=near window edge), Refresh | If scanWindow.min shifts past "from": picker shows clamped "from" immediately. No stale date visible. |
| A11 | **No dead data** | In useUsage `load` callback, add temporary `console.assert(data.records.every(r => r.date >= data.scanned_from && r.date <= data.scanned_until), "record outside scan window", data.records.map(r => r.date))` → check DevTools Console for assertion failure | Console 无 assertion failure。验证后删除临时断言。 |
| A12 | Inverted range (dev tools) | Set customFrom > customTo via React DevTools | `effectiveCustom` swaps. No crash. |
| A13 | Empty date input | Clear a date input (if browser allows) | `clampToWindow` falls back to window.max. `setCustomRange` dispatches clamped value. |
| A14 | **Window edge** | Select scanned_from as "from" date | Data shown (fully scanned day). Not partial. |
| A15 | **Type safety** | In IDE, try `setTimeRange("custom")` | TypeScript error: `"custom"` not assignable to `PresetRange`. Compile fails. |
| A16 | **Loading 禁用** | 冷启动后立即尝试点击 Custom / 切换 preset | 按钮不可点击 (opacity-50 + pointer-events-none)。数据返回后恢复交互。 |
| A17 | **Range before window** | 通过 React DevTools 将 customFrom/customTo 都设为 scanWindow.min 之前的日期 | `clampToWindow` 双向 clamp 坍缩到 win.min..win.min。picker 显示 scanned_from。过滤结果为空或仅含该日数据。 |

- [ ] **Step 7: Commit**

```bash
git add src/features/usage/components/TimeRangeTab.tsx src/features/usage/pages/UsagePage.tsx src/features/usage/CLAUDE.md src-tauri/src/features/usage/CLAUDE.md
git commit -m "feat(usage): wire custom date range with PresetRange type safety and scan window"
```

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| v1 | 2026-03-12 | Initial plan: 4 tasks, 5 files |
| v2 | 2026-03-12 | Data-driven bounds, dateRange normalization, switchToCustom, 9-scenario matrix |
| v3 | 2026-03-12 | Scan window (scanned_from) vs records boundary, clampToWindow, 12-scenario matrix |
| v4 | 2026-03-12 | effectiveCustom memo (auto-clamp on refresh), scanned_from=now-30d, records.retain(), 14-scenario matrix |
| v5 | 2026-03-12 | PresetRange type safety (setTimeRange/onChange only accept presets). scan_window_dates() + retain tests (2 new). A11 改用 console.log. parser/CLAUDE.md 纳入更新. 15-scenario matrix. |
| v6 | 2026-03-12 | Loading 禁用 TimeRangeTab (防假 scanWindow 初始化 custom). LOOKBACK_DAYS 单一真相源 (mod.rs → sub-modules via super::). A3 修正 (from=29d ago, 非 scanWindow.min). 16-scenario matrix (A16: loading guard). |
| v7 | 2026-03-12 | scan_window_dates 修正 `*now` → `now.clone()` (chrono 0.4 DateTime 非 Copy). Loading 保护下沉到 hook: switchToCustom/setCustomRange 加 `!state.data` 早退 (双层防线). |
| v8 | 2026-03-12 | A11 改用 console.assert 硬断言 (锁住 no-dead-data 不变量). Task 1 验证命令修正: subshell 隔离 cd + pnpm typecheck 先行. |
| v9 | 2026-03-12 | clampToWindow 修复: 加 `t < win.min` floor clamp, 防止两端都早于窗口时产生窗口外结果. |
| v10 | 2026-03-12 | A17: 补 range-before-window 回归场景, 锁住 clampToWindow 下界坍缩分支. 17-scenario matrix. |
