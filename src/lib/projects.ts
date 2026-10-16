import { redirect } from "@tanstack/react-router"
import { open } from "@tauri-apps/plugin-dialog"
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
  steps: number
  problem: string | null
}

export const getProjectState = () => trackedInvoke<ProjectState>("project_state")
export const openProject = (path: string) =>
  trackedInvoke<ProjectState>("open_project", { path })
export const removeRecentProject = (path: string) =>
  trackedInvoke<ProjectState>("remove_recent", { path })
export const listWorkflows = (root: string) =>
  trackedInvoke<WorkflowSummary[]>("list_workflows", { root })

export async function pickFolder() {
  const path = await open({ directory: true, multiple: false })
  return typeof path === "string" ? path : null
}

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
