import { createElement as h } from "react"
import { renderToStaticMarkup } from "react-dom/server"
import { expect, test } from "vitest"
import type { WorkflowStep } from "@/lib/projects"
import { StepCard } from "./step-card"
import { stepKindLabels } from "./step-icon"

test("workflow card identifies the called workflow", () => {
  const step = {
    id: "frontend", kind: "workflow", call: { workflow: "react-component", inputs: {}, outputs: {} },
    originalId: "frontend", label: "frontend", title: null, agent: null, execution: null, task: null,
    inputs: [], outputs: [], routes: [], workers: [],
  } satisfies WorkflowStep
  expect(stepKindLabels[step.kind]).toBe("Workflow")
  const html = renderToStaticMarkup(h(StepCard, { step, issues: 0, highlight: false }))
  expect(html).toContain("calls react-component")
  expect(html).toContain("Workflow")
  expect(html).toContain("lucide-workflow")
})

test.each([
  { title: "Approve the fix?", inputs: ["plan.md"], subtitle: "Approve the fix?" },
  { title: null, inputs: ["plan.md"], subtitle: "Decision on plan.md" },
  { title: null, inputs: [], subtitle: "Human decision" },
  { title: "  ", inputs: [], subtitle: "Human decision" },
])("human gate with title $title and inputs $inputs shows $subtitle", ({ title, inputs, subtitle }) => {
  const step = {
    id: "approve", kind: "human-gate", originalId: "approve", label: "approve",
    title, inputs, agent: null, execution: null, task: null, outputs: [], routes: [], workers: [],
  } satisfies WorkflowStep
  const html = renderToStaticMarkup(h(StepCard, { step, issues: 0, highlight: false }))
  expect(html).toContain(`>${subtitle}</p>`)
})
