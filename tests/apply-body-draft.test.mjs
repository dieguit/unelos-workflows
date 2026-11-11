import { test } from "node:test"
import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import { applyBodyDraft } from "../src/lib/apply-body-draft.ts"

test("Apply updates only the draft and closes the editor", () => {
  let draft = "before"
  let persisted = "before"
  let closed = false
  const save = () => { persisted = draft }
  applyBodyDraft("after", (body) => { draft = body }, () => { closed = true })
  assert.equal(draft, "after")
  assert.equal(persisted, "before")
  assert.equal(closed, true)
  save()
  assert.equal(persisted, "after")
})

test("every nested editor applies to its form instead of submitting it", async () => {
  for (const path of [
    "src/routes/agents/index.tsx",
    "src/components/tasks-page.tsx",
    "src/routes/skills/index.tsx",
    "src/routes/workflows/-components/workflow-editor.tsx",
  ]) {
    const source = await readFile(path, "utf8")
    assert.match(source, /onSave=\{async \(\) => \{\s*applyBodyDraft\([^\n]*setFieldValue/)
  }
  const editor = await readFile("src/components/body-editor.tsx", "utf8")
  assert.match(editor, /"Apply"/)
  assert.doesNotMatch(editor, /Save and close/)
})
