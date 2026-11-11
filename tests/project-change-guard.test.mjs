import assert from "node:assert/strict"
import { test } from "node:test"
import {
  registerProjectChangeGuard,
  requestProjectChange,
} from "../src/lib/project-change-guard.ts"

test("project change guard can deny, approve and unregister safely", async () => {
  let released = false
  const removeOld = registerProjectChangeGuard(async () => null)
  assert.equal(await requestProjectChange(), null)
  const remove = registerProjectChangeGuard(async () => () => {
    released = true
  })
  removeOld()
  const release = await requestProjectChange()
  assert.ok(release)
  release()
  assert.equal(released, true)
  remove()
  assert.equal(typeof (await requestProjectChange()), "function")
})
