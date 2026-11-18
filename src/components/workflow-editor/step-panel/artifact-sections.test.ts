import { expect, test } from "vitest"
import type { WorkflowStep } from "@/lib/projects"
import { inputSources } from "./artifact-sections"

const step = (id: string, kind: WorkflowStep["kind"], extra: Partial<WorkflowStep> = {}): WorkflowStep => ({
  id, originalId: id, label: id, kind, title: null, agent: null, execution: null,
  task: null, inputs: [], outputs: [], workers: [], routes: [], ...extra,
})

for (const kind of ["task", "human-gate", "parallel"] as const) {
  test(`${kind} can bind a declared workflow input without a producer`, () => {
    const steps = [step("consume", kind)]
    expect([...inputSources(steps, 0, ["brief.md"])]).toEqual([["brief.md", null]])
  })
}

test("input sources deduplicate declarations and preserve upstream producer labels", () => {
  const steps = [
    step("plan", "task", { outputs: ["plan.md"], routes: [{ outcome: null, target: "consume" }] }),
    step("consume", "human-gate", { inputs: ["plan.md", "brief.md", "removed.md"] }),
  ]
  const sources = inputSources(steps, 1, ["brief.md", "brief.md", "plan.md"])
  expect([...sources]).toEqual([["brief.md", null], ["plan.md", "plan"]])
  expect([...sources].filter(([name]) => !steps[1].inputs.includes(name))).toEqual([])
  expect(steps[1].inputs).toEqual(["plan.md", "brief.md", "removed.md"])
  expect([...inputSources(steps, 1, [])]).toEqual([["plan.md", "plan"]])
})
