import { redirect } from "@tanstack/react-router"
import { trackedInvoke } from "./tauri"

export type RecentProject = {
  path: string
  name: string
  openedAt: number
  available: boolean
}
export type ProjectState = { home: string; recents: RecentProject[] }
export type WorkflowSummary = {
  name: string
  file: string
  description: string | null
  steps: number | null
  problem: string | null
}
/** An alias from the global config, or a model pinned to one harness. */
export type AgentModel = string | { harness: string; model: string }
export type AgentDocument = {
  id: string
  instructions: string
  skills: string[]
  mcps: string[]
  tools: string[]
  model: AgentModel | null
}
export type AgentModelOptions = {
  aliases: string[]
  harnesses: {
    id: string
    name: string
    models: string[]
    error: string | null
  }[]
}
export type AgentSummary = {
  name: string
  model: AgentModel | null
  skills: string[]
  mcps: string[]
  problem: string | null
}
export type SkillSummary = {
  name: string
  description: string | null
  problem: string | null
}
export type SkillDocument = {
  name: string
  description: string | null
  body: string
}
export type SkillSaveResult = {
  skill: SkillDocument
  updatedReferences: string[]
}

export const getProjectState = () => trackedInvoke<ProjectState>("project_state")
export const openProject = (path: string) =>
  trackedInvoke<ProjectState>("open_project", { path })
export const removeRecentProject = (path: string) =>
  trackedInvoke<ProjectState>("remove_recent", { path })
export const listWorkflows = (root: string) =>
  trackedInvoke<WorkflowSummary[]>("list_workflows", { root })
export const listAgents = (root: string) =>
  trackedInvoke<AgentSummary[]>("list_agents", { root })
export const getAgent = (root: string, id: string) =>
  trackedInvoke<AgentDocument>("get_agent", { root, id })
export const saveAgent = (
  root: string,
  originalId: string | null,
  agent: AgentDocument
) => trackedInvoke<AgentDocument>("save_agent", { root, originalId, agent })
/** Slow: asks every harness for its models. */
export const agentModelOptions = () =>
  trackedInvoke<AgentModelOptions>("agent_model_options")
export const listSkills = (root: string) =>
  trackedInvoke<SkillSummary[]>("list_skills", { root })
export const getSkill = (root: string, id: string) =>
  trackedInvoke<SkillDocument>("get_skill", { root, id })
export const saveSkill = (
  root: string,
  id: string,
  skill: { name: string; description: string; body: string }
) => trackedInvoke<SkillSaveResult>("save_skill", { root, id, ...skill })

export const pickProject = () => trackedInvoke<ProjectState | null>("pick_project")

export function requireProject({
  context,
}: {
  context: { project: RecentProject | null }
}) {
  if (!context.project) throw redirect({ to: "/" })
  return { project: context.project }
}

/** "~/…/unelos-app" (keep=1) or "~/projects/…/unelos-app" (keep=2); short paths unchanged. */
export function shortPath(path: string, home: string, keep = 1) {
  const display =
    home && (path === home || path.startsWith(home + "/"))
      ? "~" + path.slice(home.length)
      : path
  const parts = display.split("/")
  if (parts.length <= keep + 2) return display
  return [...parts.slice(0, keep), "…", parts[parts.length - 1]].join("/")
}

const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" })
export function relativeTime(ms: number, now = Date.now()) {
  const minutes = Math.round((ms - now) / 60000)
  const [value, unit]: [number, Intl.RelativeTimeFormatUnit] =
    Math.abs(minutes) < 60
      ? [minutes, "minute"]
      : Math.abs(minutes) < 1440
        ? [Math.round(minutes / 60), "hour"]
        : [Math.round(minutes / 1440), "day"]
  const text = rtf.format(value, unit)
  return text.charAt(0).toUpperCase() + text.slice(1)
}
