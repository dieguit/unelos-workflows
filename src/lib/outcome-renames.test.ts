import { expect, test } from "vitest"
import { outcomeRenames } from "@/lib/outcome-renames.ts"
import { reassignRoute } from "@/lib/step-edit.ts"

test("renames follow original identities, including removals, additions and swaps", () => {
  expect(outcomeRenames(["good", "bad"], ["goodss", "bad"])).toEqual({ good: "goodss" })
  expect(outcomeRenames(["bad", null], ["worse", "new"])).toEqual({ bad: "worse" })
  expect(outcomeRenames(["good", "bad"], ["bad", "good"])).toEqual({ good: "bad", bad: "good" })
  expect(outcomeRenames(["bad"], ["bad"])).toEqual({})
})

test("explicit reassignment keeps destinations and refuses collisions", () => {
  const routes = [{ outcome: "good", target: "done" }, { outcome: "bad", target: "retry" }]
  expect(reassignRoute(routes, 0, "goodss")).toEqual([{ outcome: "goodss", target: "done" }, routes[1]])
  expect(reassignRoute(routes, 0, "bad")).toEqual(routes)
  expect(reassignRoute(routes, 0, null)[0]).toEqual({ outcome: null, target: "done" })
})
