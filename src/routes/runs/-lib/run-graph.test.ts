import { expect, test } from "vitest"
import { executionsByNode, missingNodes, nodeStatus } from "./run-graph"
import type { ExecutionRecord, RunState } from "./runs-api"

const rec = (nodeId: string, executionId: string): ExecutionRecord => ({ nodeId, executionId, inputs: [], outputs: [], workerReports: [], feedback: null })
const run: RunState = {
  runId: "r",
  name: "r",
  workflowName: "w",
  request: "",
  current: { kind: "task", ...rec("build", "e3") },
  history: [
    { work: { kind: "task", ...rec("build", "e1") }, outcome: "retry", reason: "bad" },
    { work: { kind: "task", ...rec("review", "e2") }, outcome: "ok", reason: null },
  ],
}

test("groups executions per node with the active one last", () => {
  const build = executionsByNode(run).get("build")
  expect(build?.map((e) => [e.n, e.active])).toEqual([[1, false], [2, true]])
})

test("status marks current, visited and untouched steps", () => {
  const s = nodeStatus(run, ["build", "review", "ship"])
  expect(s.get("build")).toEqual({ state: "current", visits: 2 })
  expect(s.get("review")).toEqual({ state: "done", visits: 1 })
  expect(s.get("ship")).toEqual({ state: "idle", visits: 0 })
})

test("waiting gate and finished end node", () => {
  expect(nodeStatus({ ...run, current: { kind: "human-gate", ...rec("gate", "g") } }, ["gate"]).get("gate")?.state).toBe("waiting")
  expect(nodeStatus({ ...run, current: { kind: "finished", nodeId: "end" } }, ["end"]).get("end")?.state).toBe("done")
})

test("reports nodes missing from the workflow", () => {
  expect(missingNodes(run, ["build"])).toEqual(["review"])
})

const childRun: RunState = {
  ...run,
  history: [
    { work: { kind: "task", ...rec("frontend/design", "e1") }, outcome: "done", reason: null },
    { work: { kind: "human-gate", ...rec("frontend/approve", "e2") }, outcome: "approved", reason: "looks good" },
  ],
  current: { kind: "task", ...rec("frontend/nested/build", "e3") },
}

test("folds child executions onto the workflow step", () => {
  const executions = executionsByNode(childRun).get("frontend")
  expect(executions?.map((e) => [e.n, e.record.nodeId, e.active])).toEqual([
    [1, "frontend/design", false],
    [2, "frontend/approve", false],
    [3, "frontend/nested/build", true],
  ])
  expect(executions?.[1]).toMatchObject({ outcome: "approved", reason: "looks good" })
  expect(nodeStatus(childRun, ["plan", "frontend", "review"]).get("frontend")).toEqual({ state: "current", visits: 3 })
})

test("waiting gate inside child marks box waiting", () => {
  const waiting: RunState = { ...childRun, current: { kind: "human-gate", ...rec("frontend/approve", "e3") } }
  expect(nodeStatus(waiting, ["frontend"]).get("frontend")).toEqual({ state: "waiting", visits: 3 })
})

test("missingNodes ignores folded ids", () => {
  expect(missingNodes(childRun, ["plan", "frontend"])).toEqual([])
  expect(missingNodes(childRun, ["plan"])).toEqual(["frontend"])
})
