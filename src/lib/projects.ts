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
export type WorkflowSettings = { name: string; description: string | null; start: string }
export type WorkflowRoute = { outcome: string | null; target: string }
export type WorkflowWorker = { title: string; agent: string }
export type StepTask = { source: "inline" | "named"; id: string | null; description: string | null; instructions: string; skills: string[]; outcomes: string[] }
export type WorkflowDraft = WorkflowSettings & { steps: WorkflowStep[] }
export type WorkflowStep = {
  id: string
  originalId: string | null
  task: StepTask | null
  label: string
  kind: "task" | "parallel" | "human-gate" | "end"
  title: string | null
  agent: string | null
  execution: "main" | "subagent" | null
  routes: WorkflowRoute[]
  workers: WorkflowWorker[]
  inputs: string[]
  outputs: string[]
}
export type WorkflowIssue = { message: string; step: string | null }
export type WorkflowDocument = WorkflowSettings & { id: string; steps: WorkflowStep[]; issues: WorkflowIssue[] }
export const getWorkflow = (root: string, id: string) =>
  trackedInvoke<WorkflowDocument>("get_workflow", { root, id })
export const validateWorkflow = (root: string, id: string, draft: WorkflowDraft) =>
  trackedInvoke<WorkflowIssue[]>("validate_workflow", { root, id, draft })
export const saveWorkflow = (root: string, id: string, draft: WorkflowDraft) =>
  trackedInvoke<WorkflowDocument>("save_workflow", { root, id, draft })
/** An alias from the global config, or a model pinned to one harness. */
export type TaskSummary = { name: string; description: string | null; skills: string[]; outcomes: string[]; problem: string | null }
export type TaskDocument = { id: string; description: string | null; instructions: string; skills: string[]; outcomes: string[] }
export const listTasks = (root: string) => trackedInvoke<TaskSummary[]>("list_tasks", { root })
export const getTask = (root: string, id: string) => trackedInvoke<TaskDocument>("get_task", { root, id })
export const saveTask = (root: string, originalId: string | null, task: TaskDocument, outcomeRenames: Record<string, string> = {}) => trackedInvoke<TaskDocument>("save_task", { root, originalId, task, outcomeRenames })
export const duplicateTask = (root: string, id: string, newId: string) => trackedInvoke<void>("duplicate_task", { root, id, newId })
export const removeTask = (root: string, id: string) => trackedInvoke<void>("remove_task", { root, id })

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
export const duplicateAgent = (root: string, id: string, newId: string) => trackedInvoke<void>("duplicate_agent", { root, id, newId })
export const duplicateSkill = (root: string, id: string, newId: string) => trackedInvoke<void>("duplicate_skill", { root, id, newId })
export const duplicateWorkflow = (root: string, id: string, newId: string) => trackedInvoke<void>("duplicate_workflow", { root, id, newId })
export const removeAgent = (root: string, id: string) => trackedInvoke<void>("remove_agent", { root, id })
export const removeSkill = (root: string, id: string) => trackedInvoke<void>("remove_skill", { root, id })
export const removeWorkflow = (root: string, id: string) => trackedInvoke<void>("remove_workflow", { root, id })
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

export const listGlobalTasks = () => trackedInvoke<TaskSummary[]>("list_global_tasks")
export const getGlobalTask = (id: string) => trackedInvoke<TaskDocument>("get_global_task", { id })
export const saveGlobalTask = (originalId: string | null, task: TaskDocument) => trackedInvoke<TaskDocument>("save_global_task", { originalId, task })
export const duplicateGlobalTask = (id: string, newId: string) => trackedInvoke<void>("duplicate_global_task", { id, newId })
export const removeGlobalTask = (id: string) => trackedInvoke<void>("remove_global_task", { id })
