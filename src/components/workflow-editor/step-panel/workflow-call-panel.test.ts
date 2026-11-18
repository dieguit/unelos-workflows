import { expect, test } from "vitest"
import type { WorkflowStep } from "@/lib/projects"
import { withCall } from "./workflow-call-panel"

test("call map edits keep producer and consumer lists synchronized", () => {
  const step = {
    id: "frontend", kind: "workflow", inputs: ["old.md"], outputs: ["old-report.md"],
    routes: [{ outcome: null, target: "review" }],
  } as WorkflowStep
  const call = {
    workflow: "child", inputs: { "brief.md": "plan.md" },
    outputs: { "report.md": "frontend-report.md" },
  }
  const edited = withCall(step, call)
  expect(edited).toEqual({
    ...step, call, inputs: ["plan.md"], outputs: ["frontend-report.md"],
  })
  expect(withCall(edited, { ...call, inputs: {}, outputs: {} }))
    .toMatchObject({ inputs: [], outputs: [], routes: step.routes })
  expect(step.inputs).toEqual(["old.md"])
})
