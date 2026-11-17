import type { StepStatus } from "@/components/flow/step-card"
import type { ExecutionRecord, RunState } from "./runs-api"

export const stepOf = (nodeId: string) => nodeId.split("/")[0]

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
    nodeId = stepOf(nodeId)
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
  for (const { work } of run.history) {
    const id = stepOf(work.nodeId)
    visits.set(id, (visits.get(id) ?? 0) + 1)
  }
  const status = new Map<string, StepStatus>()
  for (const id of stepIds) status.set(id, { state: visits.has(id) ? "done" : "idle", visits: visits.get(id) ?? 0 })
  const { current } = run
  const id = stepOf(current.nodeId)
  if (current.kind === "finished") status.set(id, { state: "done", visits: (visits.get(id) ?? 0) + 1 })
  else
    status.set(id, {
      state: current.kind === "human-gate" ? "waiting" : "current",
      visits: (visits.get(id) ?? 0) + 1,
    })
  return status
}

/** Node ids the run touched that the live workflow no longer has. */
export function missingNodes(run: RunState, stepIds: Iterable<string>) {
  const known = new Set(stepIds)
  return [...new Set([...run.history.map((h) => h.work.nodeId), run.current.nodeId].map(stepOf))].filter((id) => !known.has(id))
}
