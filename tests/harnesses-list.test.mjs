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
    "/src/routes/harnesses/-lib/harnesses-list.ts"
  )
})

after(async () => {
  await server.close()
})

const h = (id, name, o = {}) => ({
  id,
  name,
  installable: false,
  detected: false,
  installed: false,
  updateAvailable: false,
  version: null,
  integrationVersion: null,
  binary: null,
  configDir: null,
  ...o,
})

const harnesses = [
  h("zeta", "Zeta", { installable: true }),
  h("claude", "Claude Code", { detected: true, version: "2.3.0" }),
  h("amp", "Amp"),
  h("beta", "Beta", { installable: true, detected: true }),
  h("opencode", "OpenCode", {
    installable: true,
    detected: true,
    installed: true,
  }),
  h("alpha", "Alpha", { installable: true }),
]

test("sortHarnesses: installed → detected → not detected → non-installable, alpha within", () => {
  assert.deepEqual(
    mod.sortHarnesses(harnesses).map((x) => x.id),
    ["opencode", "beta", "alpha", "zeta", "claude", "amp"]
  )
})

test("filterHarnesses matches name or id, case-insensitive, trimmed, keeps order", () => {
  const ids = (q) => mod.filterHarnesses(harnesses, q).map((x) => x.id)
  assert.deepEqual(ids(" CLAUDE "), ["claude"])
  assert.deepEqual(ids("code"), ["opencode", "claude"])
  assert.deepEqual(ids("a").slice(0, 2), ["beta", "alpha"])
  assert.deepEqual(ids("nope"), [])
  assert.equal(ids("").length, harnesses.length)
})
