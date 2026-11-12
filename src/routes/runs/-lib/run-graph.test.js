import { expect, test } from "bun:test"
import { executionsByNode, missingNodes, nodeStatus } from "./run-graph"

const rec = (nodeId, executionId) => ({ nodeId, executionId, inputs: [], outputs: [], workerReports: [], feedback: null })
const run = {
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
  expect(build.map((e) => [e.n, e.active])).toEqual([[1, false], [2, true]])
})

test("status marks current, visited and untouched steps", () => {
  const s = nodeStatus(run, ["build", "review", "ship"])
  expect(s.get("build")).toEqual({ state: "current", visits: 2 })
  expect(s.get("review")).toEqual({ state: "done", visits: 1 })
  expect(s.get("ship")).toEqual({ state: "idle", visits: 0 })
})

test("waiting gate and finished end node", () => {
  expect(nodeStatus({ ...run, current: { kind: "human-gate", ...rec("gate", "g") } }, ["gate"]).get("gate").state).toBe("waiting")
  expect(nodeStatus({ ...run, current: { kind: "finished", nodeId: "end" } }, ["end"]).get("end").state).toBe("done")
})

test("reports nodes missing from the workflow", () => {
  expect(missingNodes(run, ["build"])).toEqual(["review"])
})
