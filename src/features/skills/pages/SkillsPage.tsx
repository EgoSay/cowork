/**
 * [INPUT]: 依赖 SkillCard, ToolFilter, ScanButton, useSkills, @/lib/types
 * [OUTPUT]: 对外提供 SkillsPage 组件（卡片网格 + 筛选 + 搜索）
 * [POS]: skills pages 的列表视图，被 App.tsx 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { SkillCard } from "../components/SkillCard"
import { ToolFilter } from "../components/ToolFilter"
import { ScanButton } from "../components/ScanButton"
import { useSkills } from "../hooks/useSkills"
import type { SkillMeta } from "@/lib/types"

interface SkillsPageProps {
  onSelectSkill: (skill: SkillMeta) => void
}

export function SkillsPage({ onSelectSkill }: SkillsPageProps) {
  const {
    skills, toolCounts, totalCount, filter, search, loading, error,
    setFilter, setSearch, rescan,
  } = useSkills()

  return (
    <div className="flex flex-col h-full">
      {/* 顶部工具栏 */}
      <div className="flex items-center gap-3 px-4 py-2.5 border-b border-border">
        <ToolFilter
          active={filter}
          counts={toolCounts}
          total={totalCount}
          onChange={setFilter}
        />
        <div className="flex-1" />
        <input
          type="text"
          placeholder="Search skills..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-48 px-2.5 py-1.5 rounded-md bg-bg-card border border-border text-xs text-text placeholder:text-text-muted focus:outline-none focus:border-text-muted"
        />
        <ScanButton loading={loading} onClick={rescan} />
      </div>

      {/* 卡片网格 */}
      <div className="flex-1 overflow-auto p-4">
        {error && (
          <div className="text-danger text-xs mb-4">Error: {error}</div>
        )}
        {skills.length === 0 && !loading && (
          <div className="flex items-center justify-center h-full text-text-muted text-sm">
            No skills found
          </div>
        )}
        <div className="grid grid-cols-3 gap-3">
          {skills.map((skill) => (
            <SkillCard
              key={skill.id}
              skill={skill}
              onClick={() => onSelectSkill(skill)}
            />
          ))}
        </div>
      </div>

      {/* 底部状态栏 */}
      <div className="px-4 py-2 border-t border-border text-[11px] text-text-muted">
        {totalCount} skills total &middot; {skills.length} shown
      </div>
    </div>
  )
}
