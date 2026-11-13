import type { WorkflowStep } from "@/lib/projects"
import { insertStep, removeStep } from "@/lib/step-edit"
import { expect, test } from "vitest"

import { workflowSettingsSchema } from "@/routes/workflows/-lib/workflow-settings-schema"

test("workflow settings validate renamed IDs without rejecting unchanged legacy names", () => {
  const schema = workflowSettingsSchema("legacy name")
  const draft = { name: "legacy name", description: null, start: "plan", steps: [] }
  for (const name of ["legacy name", "Workflow_2", "a".repeat(128)])
    expect(schema.safeParse({ ...draft, name }).success, name).toBeTruthy()
  for (const name of ["", "new name", "a/b", "a.b", "é", "a".repeat(129)])
    expect(schema.safeParse({ ...draft, name }).success, name).toBe(false)
  expect(schema.safeParse({ ...draft, start: "" }).success).toBe(false)
  expect(schema.safeParse({ ...draft, description: "" }).success).toBeTruthy()
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

test("removal exposes missing start and route destinations for repair", () => {
  const removed = removeStep(flow, "implement", "implement")
  const result = workflowSettingsSchema("x").safeParse({ name: "x", description: null, ...removed })
  expect(result.success).toBe(false)
  const paths = result.error!.issues.map((issue) => issue.path.join("."))
  expect(paths.includes("start")).toBeTruthy()
  expect(paths.includes("steps.0.routes.0.target")).toBeTruthy()
  expect(paths.includes("steps.1.routes.1.target")).toBeTruthy()
})

test("schema flags new task fields by path", () => {
  const { steps } = insertStep(flow, 1, "task")
  steps[2].routes = [{ outcome: "ok", target: "" }, { outcome: "ok", target: "done" }]
  const result = workflowSettingsSchema("x").safeParse({ name: "x", description: null, start: "plan", steps })
  expect(result.success).toBe(false)
  expect(result.error!.issues.map((i) => [i.path.join("."), i.message])).toEqual([
      ["steps.2.agent", "Choose an agent."],
      ["steps.2.execution", "Choose where it runs."],
      ["steps.2.routes.0.target", "Choose a destination for ok."],
      ["steps.2.routes.1.outcome", "Outcome names must be unique."],
    ])
  expect(workflowSettingsSchema("x").safeParse({ name: "x", description: null, start: "plan", steps: flow }).success).toBe(true)
})
