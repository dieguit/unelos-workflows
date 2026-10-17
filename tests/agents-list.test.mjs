import assert from "node:assert/strict"
import { after, before, test } from "node:test"
import { resolve } from "node:path"
import { createServer } from "vite"
import { createElement } from "react"
import { renderToStaticMarkup } from "react-dom/server"

let server
let mod

before(async () => {
  server = await createServer({
    configFile: false,
    resolve: { alias: { "@": resolve("src") } },
    esbuild: { jsx: "automatic" },
    server: { middlewareMode: true },
  })
  mod = await server.ssrLoadModule(
    "/src/routes/agents/-components/agents-list.tsx"
  )
})

after(async () => {
  await server.close()
})

const agents = [
  { name: "engineer", model: "provider/fast", skills: ["rust"], mcps: ["github"], problem: null },
  { name: "reviewer", model: null, skills: [], mcps: [], problem: null },
]
const broken = { name: "broken", model: null, skills: [], mcps: [], problem: "bad <yaml>" }
const render = (agents, query = "") =>
  renderToStaticMarkup(
    createElement(mod.AgentsList, { agents, query, onClear() {} })
  )

test("filterAgents matches names only, case-insensitive, trimmed", () => {
  assert.deepEqual(
    mod.filterAgents(agents, " ENGINE ").map((a) => a.name),
    ["engineer"]
  )
  for (const query of ["provider/fast", "rust", "github"])
    assert.deepEqual(mod.filterAgents(agents, query), [])
})

test("table renders columns, placeholders and no interactive chrome", () => {
  const html = render(agents)
  for (const heading of ["Name", "Model", "Skills", "MCPs"])
    assert.match(html, new RegExp(heading))
  assert.match(html, /Not configured/)
  assert.match(html, /—/)
  const rows = [...html.matchAll(/<tr\b[^>]*data-row[^>]*>/g)]
  assert.equal(rows.length, 2)
  for (const [tr] of rows) assert.match(tr, /tabindex="0"/)
  assert.match(html, /class="[^"]*text-base[^"]*font-medium[^"]*">engineer<\/span>/)
  assert.match(html, /class="[^"]*\buppercase\b[^"]*"[^>]*>Name<\/th>/)
  assert.doesNotMatch(html, /<a\b|New Agent|<script\b|aria-label="Problem/)
})

test("broken agent is its own row with a warning, not a notice", () => {
  const html = render([broken, ...agents])
  assert.equal([...html.matchAll(/<tr\b[^>]*data-row/g)].length, 3)
  assert.match(html, /aria-label="Problem: bad &lt;yaml&gt;"/)
  assert.equal([...html.matchAll(/Not configured/g)].length, 1, "broken row shows — not Not configured")
  assert.doesNotMatch(html, /with problems|<section\b/)
})

test("empty agents shows the no agents state", () => {
  const html = render([])
  assert.match(html, /No agents yet/)
  assert.doesNotMatch(html, /<table\b/)
})

test("no search matches offers clearing", () => {
  const html = render(agents, "nothing")
  assert.match(html, /No agents match/)
  assert.match(html, /Clear search/)
  assert.doesNotMatch(html, /<table\b/)
})

test("names and references render as text, long names stay non-links", () => {
  const name = "<script>x</script>" + "a".repeat(200)
  const html = render([
    { name, model: "<b>m</b>", skills: ["<i>s</i>"], mcps: ["<u>g</u>"], problem: null },
  ])
  assert.match(html, /&lt;script&gt;x&lt;\/script&gt;a{200}/)
  assert.match(html, /&lt;b&gt;m&lt;\/b&gt;/)
  assert.doesNotMatch(html, /<a\b|<script\b|<b>|<i>|<u>/)
})
