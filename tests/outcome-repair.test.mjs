import assert from "node:assert/strict"
import { test } from "node:test"
import { outcomeRenames } from "../src/lib/outcome-renames.ts"
import { reassignRoute } from "../src/routes/workflows/-lib/step-edit.ts"

test("renames follow original identities, including removals, additions and swaps", () => {
  assert.deepEqual(outcomeRenames(["good", "bad"], ["goodss", "bad"]), { good: "goodss" })
  assert.deepEqual(outcomeRenames(["bad", null], ["worse", "new"]), { bad: "worse" })
  assert.deepEqual(outcomeRenames(["good", "bad"], ["bad", "good"]), { good: "bad", bad: "good" })
  assert.deepEqual(outcomeRenames(["bad"], ["bad"]), {})
})

test("explicit reassignment keeps destinations and refuses collisions", () => {
  const routes = [{ outcome: "good", target: "done" }, { outcome: "bad", target: "retry" }]
  assert.deepEqual(reassignRoute(routes, 0, "goodss"), [{ outcome: "goodss", target: "done" }, routes[1]])
  assert.deepEqual(reassignRoute(routes, 0, "bad"), routes)
  assert.deepEqual(reassignRoute(routes, 0, null)[0], { outcome: null, target: "done" })
})
