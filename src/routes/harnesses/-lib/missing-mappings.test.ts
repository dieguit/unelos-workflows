import { expect, test } from "vitest"
import { missingAliases } from "./missing-mappings"

const agent = (name: string, model: string) => ({ name, model, skills: [], mcps: [], problem: null })

test("skips mapped aliases, puts used ones first", () => {
  const m = { a: {}, b: { claude: { model: "x" } }, c: {}, d: { opencode: { model: "y" } } }
  const agents = [agent("P", "d"), agent("Q", "d")]
  expect(missingAliases(m, "claude", agents)).toEqual([
    { alias: "d", agents: ["P", "Q"] },
    { alias: "a", agents: [] },
    { alias: "c", agents: [] },
  ])
  expect(missingAliases(m, "claude", null).map((r) => r.alias)).toEqual(["a", "c", "d"])
  expect(missingAliases({ b: { claude: { model: "x" } } }, "claude", null)).toEqual([])
})
