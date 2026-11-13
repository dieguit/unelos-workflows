import * as mod from "@/routes/model-mappings/-lib/model-mappings-list"
import { expect, test } from "vitest"


test("aliasError accepts lowercase dotted/dashed names only", () => {
  for (const ok of ["smart", "gpt-5", "a.b_c-1"])
    expect(mod.aliasError(ok)).toBe(undefined)
  for (const bad of ["", "Smart", "-a", "a-", "a--b", "a b"])
    expect(mod.aliasError(bad), bad).toBeTruthy()
})

test("filterMappings sorts and matches alias or model", () => {
  const m = { smart: { opencode: "anthropic/x" }, fast: { opencode: "openai/y" }, deep: {} }
  expect(mod.filterMappings(m, "")).toEqual(["deep", "fast", "smart"])
  expect(mod.filterMappings(m, "OPENAI")).toEqual(["fast"])
  expect(mod.filterMappings(m, "dee")).toEqual(["deep"])
})

test("usedBy is null without a project", () => {
  const agents = [
    { name: "Planner", model: "smart", skills: [], mcps: [], problem: null },
    { name: "Eng", model: "fast", skills: [], mcps: [], problem: null },
  ]
  expect(mod.usedBy(agents, "smart")).toEqual(["Planner"])
  expect(mod.usedBy(null, "smart")).toBe(null)
  expect(mod.usedByLabel(null)).toBe("—")
  expect(mod.usedByLabel(["a", "b"])).toBe("2 agents")
})
