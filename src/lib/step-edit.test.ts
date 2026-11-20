import { expect, test } from "vitest"
import { humanize, insertStep, producers, removeStep, renameStep, routesForOutcomes, slugify, stepNameError, taskLabel, taskRef, withDerivedOutcomes } from "@/lib/step-edit.ts"

import type { WorkflowDocument, WorkflowStep, WorkflowSummary } from "./projects"
import { callableWorkflows, childContract } from "./step-edit"

test("callableWorkflows excludes self and cycles", () => {
  const summaries = [
    { name: "a", calls: ["b"] }, { name: "b", calls: ["c"] },
    { name: "c", calls: [] }, { name: "d", calls: ["a"] },
  ] as WorkflowSummary[]
  expect(callableWorkflows("a", summaries)).toEqual(["b", "c"])
  expect(callableWorkflows("c", summaries)).toEqual([])
  expect(callableWorkflows("x", [
    { name: "y", calls: ["z"] }, { name: "z", calls: ["y"] },
  ] as WorkflowSummary[])).toEqual(["y", "z"])
})

test("childContract derives inputs ends artifacts", () => {
  expect(childContract({
    inputs: ["brief.md"],
    steps: [
      step("build", "task", [], { outputs: ["report.md", "shared.md"] }),
      step("nested", "workflow", [], { outputs: ["shared.md", "nested.md"] }),
      step("done", "end"), step("abandoned", "end"),
    ],
  } as WorkflowDocument)).toEqual({
    inputs: ["brief.md"], ends: ["done", "abandoned"],
    artifacts: ["report.md", "shared.md", "nested.md"],
  })
})

test("insertStep adds a workflow step", () => {
  const result = insertStep(flow, 0, "workflow")
  expect(result.index).toBe(1)
  expect(result.steps[1]).toMatchObject({
    id: "new-workflow", kind: "workflow",
    call: { workflow: "", inputs: {}, outputs: {} }, routes: [], inputs: [], outputs: [],
  })
  expect(result.steps[0].routes[0].target).toBe("new-workflow")
  expect(flow[0].routes[0].target).toBe("implement")
})

const step = (id: string, kind: WorkflowStep["kind"], routes: [string | null, string][] = [], extra: Partial<WorkflowStep> = {}): WorkflowStep => ({
  id, originalId: id, label: id, kind, title: null, agent: "engineer", execution: "main",
  routes: routes.map(([outcome, target]) => ({ outcome, target })),
  workers: [], inputs: [], outputs: [], task: kind === "task" ? { source: "inline", id: null, description: null, instructions: "x", skills: [], outcomes: [] } : null,
  ...extra,
})
const flow = [
  step("plan", "task", [[null, "implement"]], { outputs: ["plan.md"] }),
  step("implement", "task", [[null, "review"]], { outputs: ["report.md"] }),
  step("review", "task", [["approved", "done"], ["changes_requested", "implement"]]),
  step("done", "end"),
]

test("names", () => {
  expect(humanize("verify-changes")).toBe("Verify changes")
  expect(slugify("  Verify  Changes! ")).toBe("verify-changes")
})

test("step name rejects blank and duplicate slugs before rename", () => {
  expect(stepNameError("  ", flow, 1)).toBe("Enter a name.")
  expect(stepNameError(" Plan ", flow, 1)).toBe("Another step already uses this name.")
  expect(stepNameError("Implement", flow, 1)).toBe(undefined)
  expect(stepNameError("Build", flow, 1)).toBe(undefined)
  const renamed = renameStep(flow, "implement", "implement", slugify("Build"))
  expect(renamed.start).toBe("build")
  expect(renamed.steps[0].routes[0].target).toBe("build")
})


test("insert task splits only the selected transition", () => {
  const { steps, index } = insertStep(flow, 1, "task")
  expect(index).toBe(2)
  expect(steps.map((s) => s.id)).toEqual(["plan", "implement", "new-task", "review", "done"])
  expect(steps[1].routes).toEqual([{ outcome: null, target: "new-task" }])
  expect(steps[2].routes).toEqual([{ outcome: null, target: "review" }])
  expect(steps[2].agent).toBe(null)
  expect(steps[2].execution).toBe(null)
  expect(steps[3].routes).toEqual(flow[2].routes) // return target unchanged
  expect(insertStep(steps, 0, "task").steps[1].id).toBe("new-task-2")
})

test("insert gate keeps approved on the split edge and leaves rejected unset", () => {
  const { steps } = insertStep(flow, 0, "human-gate")
  expect(steps[1].routes).toEqual([{ outcome: "approved", target: "implement" }, { outcome: "rejected", target: "" }])
})

test("rename updates references and start", () => {
  const r = renameStep(flow, "implement", "implement", "build")
  expect(r.start).toBe("build")
  expect(r.steps.map((s) => s.id)).toEqual(["plan", "build", "review", "done"])
  expect(r.steps[0].routes[0].target).toBe("build")
  expect(r.steps[2].routes[1].target).toBe("build")
  expect(r.steps[1].originalId).toBe("implement")
})

test("remove clears incoming routes and orphaned artifact inputs without reconnecting or mutating", () => {
  const steps = [
    step("source", "task", [[null, "gate"]], { outputs: ["gone", "shared"] }),
    step("gate", "human-gate", [["approved", "source"], ["rejected", "done"]], { inputs: ["gone", "shared", "source"] }),
    step("parallel", "parallel", [[null, "source"]], { inputs: ["gone", "shared"], outputs: ["shared"], workers: [{ task: { source: "inline", id: null, description: null, instructions: "source", skills: [], outcomes: [] }, agent: "source" }] }),
    step("done", "end"),
  ]
  const before = structuredClone(steps)
  const result = removeStep(steps, "source", "source")
  expect(result.start).toBe("")
  expect(result.steps.map((s) => s.id)).toEqual(["gate", "parallel", "done"])
  expect(result.steps[0].routes).toEqual([{ outcome: "approved", target: "" }, { outcome: "rejected", target: "done" }])
  expect(result.steps[0].inputs).toEqual(["shared", "source"])
  expect(result.steps[1].inputs).toEqual(["shared"])
  expect(result.steps[1].routes).toEqual([{ outcome: null, target: "" }])
  expect(result.steps[1].workers).toEqual(steps[2].workers)
  expect(steps).toEqual(before)
  expect(removeStep(steps, "source", "gate").start).toBe("source")
  expect(removeStep(steps, "source", "missing")).toEqual({ steps, start: "source" })
})



test("derived outcomes and producers", () => {
  expect(withDerivedOutcomes(flow)[2].task?.outcomes).toEqual(["approved", "changes_requested"])
  expect(producers(flow, 2)).toEqual([
    { artifact: "plan.md", step: "plan" },
    { artifact: "report.md", step: "implement" },
  ])
})

test("named task routes follow its outcomes", () => {
  const routes = flow[2].routes
  expect(routesForOutcomes(routes, ["approved", "blocked"])).toEqual([
    { outcome: "approved", target: "done" },
    { outcome: "blocked", target: "" },
  ])
  expect(routesForOutcomes(routes, [])).toEqual([{ outcome: null, target: "done" }])
  expect(routesForOutcomes([{ outcome: null, target: "x" }], [])).toEqual([{ outcome: null, target: "x" }])
})

test("global task refs", () => {
  expect(taskRef("global/review")).toEqual({ global: true, name: "review" })
  expect(taskRef("review")).toEqual({ global: false, name: "review" })
  expect(taskLabel("global/review")).toBe("Global task review")
  expect(taskLabel("review")).toBe("Task review")
})
