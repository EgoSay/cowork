/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 TAG_OPTIONS, TagId, relativeTime, computeDistribution, formatTime, formatDate, localDateString, yesterdayString, DistributionItem
 * [POS]: projects 工具函数，被 hooks 和 components 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */

// ── 标签常量 ────────────────────────────────────────

export const TAG_OPTIONS = [
  { id: "efficient", label: "高效", color: "text-success" },
  { id: "pitfall", label: "踩坑", color: "text-danger" },
  { id: "template", label: "模板", color: "text-[#818cf8]" },
] as const

export type TagId = typeof TAG_OPTIONS[number]["id"]

// ── 相对时间（接受 ISO 时间戳字符串） ──────────────

export function relativeTime(isoStr: string): string {
  const ts = new Date(isoStr).getTime()
  if (isNaN(ts)) return ""
  const diff = (Date.now() - ts) / 1000
  if (diff < 60) return "刚刚"
  if (diff < 3600) return `${Math.floor(diff / 60)}分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)}小时前`
  if (diff < 604800) return `${Math.floor(diff / 86400)}天前`
  return new Date(ts).toLocaleDateString()
}

// ── 时间格式化（接受 ISO 时间戳字符串） ────────────

export function formatTime(isoStr: string): string {
  const d = new Date(isoStr)
  if (isNaN(d.getTime())) return ""
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
}

export function formatDate(isoStr: string): string {
  const d = new Date(isoStr)
  if (isNaN(d.getTime())) return ""
  return `${d.getMonth() + 1}/${d.getDate()}`
}

// ── 日期工具 ────────────────────────────────────────

export function localDateString(isoStr?: string): string {
  const d = isoStr ? new Date(isoStr) : new Date()
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, "0")
  const day = String(d.getDate()).padStart(2, "0")
  return `${y}-${m}-${day}`
}

export function yesterdayString(): string {
  const d = new Date()
  d.setDate(d.getDate() - 1)
  return localDateString(d.toISOString())
}

// ── 时间分配比例 ──────────────────────────────────

export interface DistributionItem {
  label: string
  count: number
  ratio: number
}

export function computeDistribution(
  items: { label: string; count: number }[],
): DistributionItem[] {
  const total = items.reduce((s, i) => s + i.count, 0)
  if (total === 0) return []
  return items
    .map(i => ({ ...i, ratio: i.count / total }))
    .filter(i => i.count > 0)
    .sort((a, b) => b.count - a.count)
}
