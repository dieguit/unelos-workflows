import { expect, test } from "vitest"
import { applyBodyDraft } from "@/lib/apply-body-draft.ts"

test("Apply updates only the draft and closes the editor", () => {
  let draft = "before"
  let persisted = "before"
  let closed = false
  const save = () => { persisted = draft }
  applyBodyDraft("after", (body) => { draft = body }, () => { closed = true })
  expect(draft).toBe("after")
  expect(persisted).toBe("before")
  expect(closed).toBe(true)
  save()
  expect(persisted).toBe("after")
})

