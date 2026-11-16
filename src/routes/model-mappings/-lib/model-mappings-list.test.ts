import * as mod from "@/routes/model-mappings/-lib/model-mappings-list"
import { usedBy, usedByLabel } from "@/lib/model-mappings"
import { expect, test } from "vitest"


test("aliasError accepts lowercase dotted/dashed names only", () => {
  for (const ok of ["smart", "gpt-5", "a.b_c-1"])
    expect(mod.aliasError(ok)).toBe(undefined)
  for (const bad of ["", "Smart", "-a", "a-", "a--b", "a b"])
    expect(mod.aliasError(bad), bad).toBeTruthy()
})

test("filterMappings sorts and matches alias or model", () => {
  const m = { smart: { opencode: "anthropic/x" }, fast: { opencode: "openai/y" }, deep: {} }
  const harnesses = [{ id: "opencode" }]
  expect(mod.filterMappings(m, "", harnesses)).toEqual(["deep", "fast", "smart"])
  expect(mod.filterMappings(m, "OPENAI", harnesses)).toEqual(["fast"])
  expect(mod.filterMappings(m, "dee", harnesses)).toEqual(["deep"])
})

test("filterMappings searches installed harnesses only, including Claude", () => {
  const mappings = { smart: { claude: "sonnet", opencode: "provider/hidden" } }
  expect(mod.filterMappings(mappings, "sonnet", [{ id: "claude" }])).toEqual(["smart"])
  expect(mod.filterMappings(mappings, "hidden", [{ id: "claude" }])).toEqual([])
  expect(mod.filterMappings(mappings, "", [])).toEqual(["smart"])
  expect(mod.filterMappings(mappings, "sonnet", [])).toEqual([])
})

test("usedBy is null without a project", () => {
  const agents = [
    { name: "Planner", model: "smart", skills: [], mcps: [], problem: null },
    { name: "Eng", model: "fast", skills: [], mcps: [], problem: null },
  ]
  expect(usedBy(agents, "smart")).toEqual(["Planner"])
  expect(usedBy(null, "smart")).toBe(null)
  expect(usedByLabel(null)).toBe("—")
  expect(usedByLabel(["a", "b"])).toBe("2 agents")
})
