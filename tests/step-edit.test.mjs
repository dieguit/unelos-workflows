import assert from "node:assert/strict"
import { test } from "node:test"
import { readFile } from "node:fs/promises"
import { humanize, insertStep, producers, renameStep, routesForOutcomes, slugify, stepNameError, taskLabel, taskRef, withDerivedOutcomes } from "../src/routes/workflows/-lib/step-edit.ts"
import { workflowSettingsSchema } from "../src/routes/workflows/-lib/workflow-settings-schema.ts"

const step = (id, kind, routes = [], extra = {}) => ({
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
  assert.equal(humanize("verify-changes"), "Verify changes")
  assert.equal(slugify("  Verify  Changes! "), "verify-changes")
})

test("step name rejects blank and duplicate slugs before rename", () => {
  assert.equal(stepNameError("  ", flow, 1), "Enter a name.")
  assert.equal(stepNameError(" Plan ", flow, 1), "Another step already uses this name.")
  assert.equal(stepNameError("Implement", flow, 1), undefined)
  assert.equal(stepNameError("Build", flow, 1), undefined)
  const renamed = renameStep(flow, "implement", "implement", slugify("Build"))
  assert.equal(renamed.start, "build")
  assert.equal(renamed.steps[0].routes[0].target, "build")
})

test("workflow save is blocked while invalid step name text is buffered", async () => {
  const editor = await readFile("src/routes/workflows/-components/workflow-editor.tsx", "utf8")
  const field = await readFile("src/routes/workflows/-components/step-panel/basic-fields.tsx", "utf8")
  assert.match(editor, /localValid && Object\.keys\(nameDrafts\)\.length === 0/)
  assert.match(editor, /nameDraft=\{nameDrafts\[step\.id\]\}/)
  assert.match(field, /onNameDraft\(from, error \? \{ text: e\.target\.value, error \} : undefined\)/)
  assert.match(field, /if \(error\) return/)
})

test("insert task splits only the selected transition", () => {
  const { steps, index } = insertStep(flow, 1, "task")
  assert.equal(index, 2)
  assert.deepEqual(steps.map((s) => s.id), ["plan", "implement", "new-task", "review", "done"])
  assert.deepEqual(steps[1].routes, [{ outcome: null, target: "new-task" }])
  assert.deepEqual(steps[2].routes, [{ outcome: null, target: "review" }])
  assert.equal(steps[2].agent, null)
  assert.equal(steps[2].execution, null)
  assert.deepEqual(steps[3].routes, flow[2].routes) // return target unchanged
  assert.equal(insertStep(steps, 0, "task").steps[1].id, "new-task-2")
})

test("insert gate keeps approved on the split edge and leaves rejected unset", () => {
  const { steps } = insertStep(flow, 0, "human-gate")
  assert.deepEqual(steps[1].routes, [{ outcome: "approved", target: "implement" }, { outcome: "rejected", target: "" }])
})

test("rename updates references and start", () => {
  const r = renameStep(flow, "implement", "implement", "build")
  assert.equal(r.start, "build")
  assert.deepEqual(r.steps.map((s) => s.id), ["plan", "build", "review", "done"])
  assert.equal(r.steps[0].routes[0].target, "build")
  assert.equal(r.steps[2].routes[1].target, "build")
  assert.equal(r.steps[1].originalId, "implement")
})

test("derived outcomes and producers", () => {
  assert.deepEqual(withDerivedOutcomes(flow)[2].task.outcomes, ["approved", "changes_requested"])
  assert.deepEqual(producers(flow, 2), [
    { artifact: "plan.md", step: "plan" },
    { artifact: "report.md", step: "implement" },
  ])
})

test("named task routes follow its outcomes", () => {
  const routes = flow[2].routes
  assert.deepEqual(routesForOutcomes(routes, ["approved", "blocked"]), [
    { outcome: "approved", target: "done" },
    { outcome: "blocked", target: "" },
  ])
  assert.deepEqual(routesForOutcomes(routes, []), [{ outcome: null, target: "done" }])
  assert.deepEqual(routesForOutcomes([{ outcome: null, target: "x" }], []), [{ outcome: null, target: "x" }])
})

test("global task refs", () => {
  assert.deepEqual(taskRef("global/review"), { global: true, name: "review" })
  assert.deepEqual(taskRef("review"), { global: false, name: "review" })
  assert.equal(taskLabel("global/review"), "Global task review")
  assert.equal(taskLabel("review"), "Task review")
})

test("schema flags new task fields by path", () => {
  const { steps } = insertStep(flow, 1, "task")
  steps[2].routes = [{ outcome: "ok", target: "" }, { outcome: "ok", target: "done" }]
  const result = workflowSettingsSchema("x").safeParse({ name: "x", description: null, start: "plan", steps })
  assert.equal(result.success, false)
  assert.deepEqual(
    result.error.issues.map((i) => [i.path.join("."), i.message]),
    [
      ["steps.2.agent", "Choose an agent."],
      ["steps.2.execution", "Choose where it runs."],
      ["steps.2.routes.0.target", "Choose a destination for ok."],
      ["steps.2.routes.1.outcome", "Outcome names must be unique."],
    ]
  )
  assert.equal(workflowSettingsSchema("x").safeParse({ name: "x", description: null, start: "plan", steps: flow }).success, true)
})
