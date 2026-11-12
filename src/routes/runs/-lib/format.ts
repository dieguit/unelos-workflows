import type { RunListItem } from "./runs-api"

const date = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" })

export const formatTime = (ms?: number | null) => (ms ? date.format(ms) : "—")

export function filterRuns(list: RunListItem[], query: string) {
  const q = query.trim().toLowerCase()
  if (!q) return list
  return list.filter((r) => [r.name, r.workflowName, r.nodeId].some((s) => s.toLowerCase().includes(q)))
}
