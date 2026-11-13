export type ImportKind = "workflow" | "agent" | "task" | "skill" | "mcp"

export type ImportConflict = {
  key: string
  kind: ImportKind
  id: string
  suggested: string
  reason: "exists" | "referencesRenamed"
  causedBy?: string
  error?: string
}

export type ImportPlan = {
  source: string
  workflow: { id: string; description?: string }
  items: { kind: ImportKind; id: string }[]
  reused: string[]
  mcps: { id: string; command?: string; url?: string }[]
  warnings: { kind: "missingGlobal" | "unresolvedModel"; message: string }[]
  conflicts: ImportConflict[]
}

export type ImportCommit =
  | { status: "imported"; workflow: string; warningCount: number }
  | { status: "conflicts"; plan: ImportPlan }

export type ImportRenames = Record<string, string>

export const kindLabel: Record<ImportKind, string> = {
  workflow: "Workflow",
  agent: "Agent",
  task: "Task",
  skill: "Skill",
  mcp: "MCP",
}

export function splitError(error: unknown) {
  const [title, ...hint] = String(error).split("\n")
  return { title, hint: hint.join("\n") }
}

export const isRemoteSource = (source: string) => /^[a-z][a-z0-9+.-]*:\/\/|^git@/i.test(source.trim())

export const hasConflictErrors = (plan: ImportPlan) => plan.conflicts.some((c) => c.error)

export const humanize = (id: string) => id.replace(/-/g, " ").replace(/^./, (c) => c.toUpperCase())

export const repoLabel = (source: string) =>
  source.match(/^(?:[a-z][a-z0-9+.-]*:\/\/|git@)[^/:]+[/:]([^/#]+\/[^/#]+?)(?:\.git)?(?:[/#?]|$)/i)?.[1]

export function causeText(causedBy: string) {
  const [kind, ...id] = causedBy.split(":")
  return `Renamed because it uses ${kind} “${id.join(":")}”, which was renamed.`
}
