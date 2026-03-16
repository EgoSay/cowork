/**
 * [INPUT]: 依赖 ../hooks/useProjects, ../components/*, @/lib/types
 * [OUTPUT]: 对外提供 ProjectsPage 主页面组件
 * [POS]: Projects 模块主页面（晨间焦点 + 热力图 + 项目列表 + 会话列表 + 闪卡 + 会话详情）
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useProjects } from "../hooks/useProjects"
import { MorningFocus } from "../components/MorningFocus"
import { TokenHeatmap } from "../components/TokenHeatmap"
import { TagFilter } from "../components/TagFilter"
import { ProjectCard } from "../components/ProjectCard"
import { SessionCard } from "../components/SessionCard"
import { FlashCard } from "../components/FlashCard"
import { SessionDetail } from "../components/SessionDetail"

interface ProjectsPageProps {
  active: boolean
}

export function ProjectsPage({ active }: ProjectsPageProps) {
  const {
    loading,
    error,
    annotations,
    selectedProjectId,
    selectedSession,
    search,
    tagFilter,
    flashSession,
    projectData,
    filteredProjects,
    filteredSessions,
    morningFocus,
    tagDistribution,
    selectProject,
    selectSession,
    setSearch,
    setTagFilter,
    dismissFlash,
    annotateSession,
  } = useProjects(active)

  // ── 会话详情所需的 dirPath 查找 ─────────────────
  const detailProject = selectedSession
    ? projectData.find(pd => pd.project.id === selectedSession.project_id)?.project
    : null
  const detailDirPath = detailProject?.dir_path ?? ""
  const detailProjectName = detailProject?.name ?? ""

  // ── 加载态 ────────────────────────────────────────
  if (loading) {
    return (
      <div className="flex items-center justify-center h-full text-text-muted text-sm">
        扫描项目中...
      </div>
    )
  }

  // ── 错误态 ────────────────────────────────────────
  if (error) {
    return (
      <div className="flex items-center justify-center h-full text-danger text-sm">
        {error}
      </div>
    )
  }

  // ── 会话详情视图 ────────────────────────────────
  if (selectedSession) {
    return (
      <SessionDetail
        session={selectedSession}
        dirPath={detailDirPath}
        projectName={detailProjectName}
        annotation={annotations[selectedSession.id]}
        onBack={() => selectSession(null)}
        onAnnotate={(tags, note) => annotateSession(selectedSession.id, tags, note)}
      />
    )
  }

  // ── Flash card 项目名查找 ─────────────────────────
  const flashProjectName = flashSession
    ? filteredProjects.find(p => p.id === flashSession.project_id)?.name ?? "未知项目"
    : ""

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* ── 晨间焦点 ──────────────────────────────── */}
      <MorningFocus
        sessionCount={morningFocus.sessionCount}
        avgTurns={morningFocus.avgTurns}
        efficient={morningFocus.efficient}
        pitfall={morningFocus.pitfall}
        distribution={tagDistribution}
      />

      {/* ── Token 热力图 ──────────────────────────── */}
      <TokenHeatmap />

      {/* ── 搜索 + 标签筛选 ──────────────────────── */}
      <div className="flex items-center gap-3 px-4 py-2 border-b border-border">
        <input
          type="text"
          value={search}
          onChange={e => setSearch(e.target.value)}
          placeholder="搜索项目..."
          className="w-48 px-3 py-1.5 rounded-lg bg-bg-card border border-border text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-text-muted/50"
        />
        <TagFilter selected={tagFilter} onChange={setTagFilter} />
      </div>

      {/* ── 主体: 项目列表 + 会话列表 ─────────────── */}
      <div className="flex flex-1 min-h-0">
        {/* 左面板: 项目列表 */}
        <div className="w-56 border-r border-border overflow-y-auto p-2 space-y-1">
          {filteredProjects.length === 0 ? (
            <div className="text-center text-text-muted text-xs py-8">
              暂无项目
            </div>
          ) : (
            filteredProjects.map(project => (
              <ProjectCard
                key={project.id}
                project={project}
                selected={project.id === selectedProjectId}
                onClick={() => selectProject(project.id)}
              />
            ))
          )}
        </div>

        {/* 右面板: 会话列表 */}
        <div className="flex-1 overflow-y-auto p-3 space-y-2">
          {!selectedProjectId ? (
            <div className="flex items-center justify-center h-full text-text-muted text-sm">
              选择一个项目查看会话
            </div>
          ) : filteredSessions.length === 0 ? (
            <div className="flex items-center justify-center h-full text-text-muted text-sm">
              无匹配会话
            </div>
          ) : (
            filteredSessions.map(session => (
              <SessionCard
                key={session.id}
                session={session}
                annotation={annotations[session.id]}
                onAnnotate={(tags, note) => annotateSession(session.id, tags, note)}
                onClick={() => selectSession(session)}
              />
            ))
          )}
        </div>
      </div>

      {/* ── Flash Card overlay ────────────────────── */}
      {flashSession && (
        <FlashCard
          session={flashSession}
          projectName={flashProjectName}
          onAnnotate={(tags, note) => {
            annotateSession(flashSession.id, tags, note)
            dismissFlash()
          }}
          onDismiss={dismissFlash}
        />
      )}
    </div>
  )
}
