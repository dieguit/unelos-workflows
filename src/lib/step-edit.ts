import type { WorkflowStep, WorkflowWorker } from "@/lib/projects"

export const humanize = (id: string) => {
  const text = id.replace(/[-_]+/g, " ").trim()
  return text.charAt(0).toUpperCase() + text.slice(1)
}

export const slugify = (name: string) =>
  name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "")

export function stepNameError(name: string, steps: WorkflowStep[], index: number) {
  const id = slugify(name)
  if (!id) return "Enter a name."
  if (steps.some((step, i) => i !== index && step.id === id))
    return "Another step already uses this name."
}

const uniqueId = (steps: WorkflowStep[], base: string) => {
  const ids = new Set(steps.map((s) => s.id))
  let id = base
  for (let n = 2; ids.has(id); n++) id = `${base}-${n}`
  return id
}

const blank = (id: string, kind: WorkflowStep["kind"]): WorkflowStep => ({
  id,
  originalId: null,
  label: id,
  kind,
  title: null,
  agent: null,
  execution: null,
  routes: [],
  workers: [],
  inputs: [],
  outputs: [],
  task: null,
})

/** Splits the transition from steps[index] to steps[index + 1] with a new step. Returns the new steps and the new step's index. */
export function insertStep(
  steps: WorkflowStep[],
  index: number,
  kind: "task" | "human-gate"
) {
  const next = steps[index + 1]?.id ?? ""
  const step = blank(uniqueId(steps, kind === "task" ? "new-task" : "approval"), kind)
  if (kind === "task") {
    step.task = { source: "inline", id: null, description: null, instructions: "", skills: [], outcomes: [] }
    step.routes = [{ outcome: null, target: next }]
  } else {
    // Destinations are never picked automatically; only the split edge is kept.
    step.routes = [
      { outcome: "approved", target: next },
      { outcome: "rejected", target: "" },
    ]
  }
  const source = steps[index]
  const updated = {
    ...source,
    routes: source.routes.map((r) => (r.target === next ? { ...r, target: step.id } : r)),
  }
  return {
    steps: [...steps.slice(0, index), updated, step, ...steps.slice(index + 1)],
    index: index + 1,
  }
}

/** Renames a step and every reference to it. Callers must ensure `to` is unused. */
export function renameStep(steps: WorkflowStep[], start: string, from: string, to: string) {
  return {
    steps: steps.map((s) => ({
      ...s,
      id: s.id === from ? to : s.id,
      label: s.id === from ? to : s.label,
      routes: s.routes.map((r) => (r.target === from ? { ...r, target: to } : r)),
    })),
    start: start === from ? to : start,
  }
}

/** Removes a step without reconnecting its incoming routes. */
export function removeStep(steps: WorkflowStep[], start: string, id: string) {
  const removed = steps.find((step) => step.id === id)
  if (!removed) return { steps, start }
  const remaining = steps.filter((step) => step.id !== id)
  const orphaned = new Set(removed.outputs.filter((output) =>
    !remaining.some((step) => step.outputs.includes(output))
  ))
  return {
    steps: remaining.map((step) => ({
      ...step,
      routes: step.routes.map((route) => route.target === id ? { ...route, target: "" } : route),
      inputs: step.inputs.filter((input) => !orphaned.has(input)),
    })),
    start: start === id ? "" : start,
  }
}

/** Inline tasks keep outcome names in their routes while editing; the definition is derived on save. */
export const withDerivedOutcomes = (steps: WorkflowStep[]) =>
  steps.map((s) =>
    s.task?.source === "inline"
      ? { ...s, task: { ...s.task, outcomes: s.routes.flatMap((r) => (r.outcome === null ? [] : [r.outcome])) } }
      : s
  )

/** A named task id as stored in the workflow: `global/<id>` for machine-wide tasks, plain for this project's. */
export const taskRef = (id: string) =>
  id.startsWith("global/") ? { global: true, name: id.slice(7) } : { global: false, name: id }

export const workerTitle = (worker: WorkflowWorker) =>
  worker.task.source === "inline" ? worker.task.description ?? "" : taskRef(worker.task.id ?? "").name

export const addWorker = (step: WorkflowStep): WorkflowStep => ({
  ...step,
  workers: [...step.workers, {
    task: { source: "inline", id: null, description: null, instructions: "", skills: [], outcomes: [] },
    agent: "",
  }],
})

/** "Global task review" / "Task review". */
export const taskLabel = (id: string) => {
  const ref = taskRef(id)
  return `${ref.global ? "Global task" : "Task"} ${ref.name}`
}

/** Routes for a named task: one per outcome (keeping matching destinations), or a single next step when it declares none. */
export const routesForOutcomes = (routes: WorkflowStep["routes"], outcomes: string[]) =>
  outcomes.length
    ? outcomes.map((outcome) => ({ outcome, target: routes.find((r) => r.outcome === outcome)?.target ?? "" }))
    : [{ outcome: null, target: (routes.find((r) => r.outcome === null) ?? routes[0])?.target ?? "" }]

export function reassignRoute(routes: WorkflowStep["routes"], index: number, outcome: string | null) {
  if (routes.some((route, i) => i !== index && route.outcome === outcome)) return routes
  return routes.map((route, i) => i === index ? { ...route, outcome } : route)
}

/** Steps upstream of `index` (any step that can reach it) that declare outputs. */
export function producers(steps: WorkflowStep[], index: number) {
  const target = steps[index]?.id
  const reaches = new Set<string>([target])
  for (let changed = true; changed; ) {
    changed = false
    for (const s of steps)
      if (!reaches.has(s.id) && s.routes.some((r) => reaches.has(r.target))) {
        reaches.add(s.id)
        changed = true
      }
  }
  reaches.delete(target)
  return steps.flatMap((s) =>
    reaches.has(s.id) ? s.outputs.map((artifact) => ({ artifact, step: s.id })) : []
  )
}
