import type { StepStatus } from "@/components/flow/step-card"
import type { ExecutionRecord, RunState } from "./runs-api"

export type Execution = {
  /** 1-based visit number of this node. */
  n: number
  record: ExecutionRecord
  outcome: string | null
  reason: string | null
  /** The run is working on this execution right now. */
  active: boolean
}

/** Executions per node id, oldest first. */
export function executionsByNode(run: RunState) {
  const map = new Map<string, Execution[]>()
  const add = (nodeId: string, record: ExecutionRecord, rest: Pick<Execution, "outcome" | "reason" | "active">) => {
    const list = map.get(nodeId) ?? []
    list.push({ n: list.length + 1, record, ...rest })
    map.set(nodeId, list)
  }
  for (const { work, outcome, reason } of run.history)
    if (work.kind !== "finished") add(work.nodeId, work, { outcome, reason, active: false })
  if (run.current.kind !== "finished")
    add(run.current.nodeId, run.current, { outcome: null, reason: null, active: true })
  return map
}

/** Status for every step in `stepIds`; untouched ones are idle. */
export function nodeStatus(run: RunState, stepIds: Iterable<string>) {
  const visits = new Map<string, number>()
  for (const { work } of run.history) visits.set(work.nodeId, (visits.get(work.nodeId) ?? 0) + 1)
  const status = new Map<string, StepStatus>()
  for (const id of stepIds) status.set(id, { state: visits.has(id) ? "done" : "idle", visits: visits.get(id) ?? 0 })
  const { current } = run
  if (current.kind === "finished") status.set(current.nodeId, { state: "done", visits: (visits.get(current.nodeId) ?? 0) + 1 })
  else
    status.set(current.nodeId, {
      state: current.kind === "human-gate" ? "waiting" : "current",
      visits: (visits.get(current.nodeId) ?? 0) + 1,
    })
  return status
}

/** Node ids the run touched that the live workflow no longer has. */
export function missingNodes(run: RunState, stepIds: Iterable<string>) {
  const known = new Set(stepIds)
  return [...new Set([...run.history.map((h) => h.work.nodeId), run.current.nodeId])].filter((id) => !known.has(id))
}
