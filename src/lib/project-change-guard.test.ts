import { expect, test } from "vitest"
import {
  registerProjectChangeGuard,
  requestProjectChange,
} from "@/lib/project-change-guard.ts"

test("project change guard can deny, approve and unregister safely", async () => {
  let released = false
  const removeOld = registerProjectChangeGuard(async () => null)
  expect(await requestProjectChange()).toBe(null)
  const remove = registerProjectChangeGuard(async () => () => {
    released = true
  })
  removeOld()
  const release = await requestProjectChange()
  expect(release).toBeTruthy()
  release!()
  expect(released).toBe(true)
  remove()
  expect(typeof (await requestProjectChange())).toBe("function")
})
