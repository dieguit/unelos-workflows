import type { WorkflowSummary } from "@/lib/projects"

export function filterWorkflows(list: WorkflowSummary[], query: string) {
  const q = query.trim().toLowerCase()
  if (!q) return list
  return list.filter((w) =>
    [w.name, w.file, w.description ?? ""].some((s) =>
      s.toLowerCase().includes(q)
    )
  )
}
