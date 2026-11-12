import { trackedInvoke } from "@/lib/tauri"

export type ArtifactReference = { name: string; path: string }
export type WorkerReportBinding = { key: string; report: ArtifactReference }
export type TransitionFeedback = {
  sourceNodeId: string
  sourceExecutionId: string
  outcome: string | null
  reason: string | null
  artifacts: ArtifactReference[]
}
export type ExecutionRecord = {
  nodeId: string
  executionId: string
  inputs: ArtifactReference[]
  outputs: ArtifactReference[]
  workerReports: WorkerReportBinding[]
  feedback: TransitionFeedback | null
}
export type CurrentWork =
  | ({ kind: "task" | "parallel" | "human-gate" } & ExecutionRecord)
  | { kind: "finished"; nodeId: string }
export type CompletedExecution = {
  work: CurrentWork
  outcome: string | null
  reason: string | null
}
/** Epoch milliseconds; absent on runs created before timestamps existed. */
export type RunState = {
  runId: string
  name: string
  workflowName: string
  request: string
  current: CurrentWork
  history: CompletedExecution[]
  /** Paths currently present in this run's artifact folder. */
  existingArtifactPaths?: string[]
  startedAt?: number | null
  updatedAt?: number | null
}
export type RunListItem = {
  runId: string
  name: string
  workflowName: string
  status: "running" | "waiting" | "finished"
  nodeId: string
  startedAt?: number | null
  updatedAt?: number | null
}

export const listRuns = (root: string) =>
  trackedInvoke<RunListItem[]>("list_runs", { root })
export const getRun = (root: string, runId: string) =>
  trackedInvoke<RunState>("get_run", { root, runId })
export const deleteRun = (root: string, runId: string) =>
  trackedInvoke<void>("delete_run", { root, runId })
/** Only paths referenced by the run's own state are readable. */
export const readRunArtifact = (root: string, runId: string, path: string) =>
  trackedInvoke<string>("read_run_artifact", { root, runId, path })
