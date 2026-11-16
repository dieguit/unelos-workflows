import { expect, test } from "vitest"
import { effortChoices, effortsFor } from "./effort-options"

test("effort choices preserve saved values and flag only known stale values", () => {
  expect(effortChoices(["low", "high"], "max")).toEqual([
    { value: "low", stale: false },
    { value: "high", stale: false },
    { value: "max", stale: true },
  ])
  expect(effortChoices(undefined, "max")).toEqual([{ value: "max", stale: false }])
  expect(effortChoices(["low"], undefined)).toEqual([{ value: "low", stale: false }])
  expect(effortChoices(["low"], "low")).toEqual([{ value: "low", stale: false }])
  expect(effortChoices([], "max")).toEqual([{ value: "max", stale: true }])
})

test("model efforts distinguish unavailable models from known empty lists", () => {
  expect(effortsFor([{ id: "a", efforts: ["x"] }], "a")).toEqual(["x"])
  expect(effortsFor([{ id: "a", efforts: [] }], "a")).toEqual([])
  expect(effortsFor([], "a")).toBeUndefined()
  expect(effortsFor(undefined, "a")).toBeUndefined()
  expect(effortsFor([], undefined)).toBeUndefined()
})
