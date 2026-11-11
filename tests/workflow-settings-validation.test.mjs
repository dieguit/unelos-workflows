import assert from "node:assert/strict"
import { test } from "node:test"
import { workflowSettingsSchema } from "../src/routes/workflows/-lib/workflow-settings-schema.ts"

test("workflow settings validate renamed IDs without rejecting unchanged legacy names", () => {
  const schema = workflowSettingsSchema("legacy name")
  const draft = { name: "legacy name", description: null, start: "plan", steps: [] }
  for (const name of ["legacy name", "Workflow_2", "a".repeat(128)])
    assert.ok(schema.safeParse({ ...draft, name }).success, name)
  for (const name of ["", "new name", "a/b", "a.b", "é", "a".repeat(129)])
    assert.equal(schema.safeParse({ ...draft, name }).success, false, name)
  assert.equal(schema.safeParse({ ...draft, start: "" }).success, false)
  assert.ok(schema.safeParse({ ...draft, description: "" }).success)
})
