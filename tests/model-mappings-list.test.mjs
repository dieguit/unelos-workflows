import assert from "node:assert/strict"
import { after, before, test } from "node:test"
import { resolve } from "node:path"
import { createServer } from "vite"

let server
let mod

before(async () => {
  server = await createServer({
    configFile: false,
    resolve: { alias: { "@": resolve("src") } },
    server: { middlewareMode: true },
  })
  mod = await server.ssrLoadModule(
    "/src/routes/model-mappings/-lib/model-mappings-list.ts"
  )
})

after(async () => {
  await server.close()
})

test("aliasError accepts lowercase dotted/dashed names only", () => {
  for (const ok of ["smart", "gpt-5", "a.b_c-1"])
    assert.equal(mod.aliasError(ok), undefined)
  for (const bad of ["", "Smart", "-a", "a-", "a--b", "a b"])
    assert.ok(mod.aliasError(bad), bad)
})

test("filterMappings sorts and matches alias or model", () => {
  const m = { smart: { opencode: "anthropic/x" }, fast: { opencode: "openai/y" }, deep: {} }
  assert.deepEqual(mod.filterMappings(m, ""), ["deep", "fast", "smart"])
  assert.deepEqual(mod.filterMappings(m, "OPENAI"), ["fast"])
  assert.deepEqual(mod.filterMappings(m, "dee"), ["deep"])
})

test("usedBy is null without a project", () => {
  const agents = [{ name: "Planner", model: "smart" }, { name: "Eng", model: "fast" }]
  assert.deepEqual(mod.usedBy(agents, "smart"), ["Planner"])
  assert.equal(mod.usedBy(null, "smart"), null)
  assert.equal(mod.usedByLabel(null), "—")
  assert.equal(mod.usedByLabel(["a", "b"]), "2 agents")
})
