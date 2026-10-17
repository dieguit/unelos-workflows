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
    "/src/routes/skills/-components/skills-list.tsx"
  )
})

after(async () => {
  await server.close()
})

const skills = [
  { name: "code-review", description: "Review code for correctness.", problem: null },
  { name: "rust", description: null, problem: null },
]
const broken = { name: "bad", description: null, problem: "bad <yaml>" }
const render = (skills, query = "") =>
  renderToStaticMarkup(
    createElement(mod.SkillsList, { skills, query, onClear() {} })
  )

test("filterSkills matches name or description, case-insensitive, trimmed", () => {
  const names = (q) => mod.filterSkills(skills, q).map((s) => s.name)
  assert.deepEqual(names(" RUST "), ["rust"])
  assert.deepEqual(names("correctness"), ["code-review"])
  assert.deepEqual(names("nothing"), [])
})

test("table renders Name and Description with placeholder", () => {
  const html = render(skills)
  assert.match(html, /class="[^"]*\buppercase\b[^"]*"[^>]*>Name<\/th>/)
  assert.match(html, />Description<\/th>/)
  assert.match(html, /Review code for correctness\./)
  assert.match(html, /—/)
  assert.equal([...html.matchAll(/<tr\b[^>]*data-row[^>]*>/g)].length, 2)
  assert.doesNotMatch(html, /<a\b|New skill|<script\b|aria-label="Problem/)
})

test("broken skill is its own row with a warning, not a notice", () => {
  const html = render([broken, ...skills])
  assert.equal([...html.matchAll(/<tr\b[^>]*data-row/g)].length, 3)
  assert.match(html, /aria-label="Problem: bad &lt;yaml&gt;"/)
  assert.doesNotMatch(html, /with problems|<section\b/)
})

test("empty skills shows the no skills state", () => {
  const html = render([])
  assert.match(html, /No skills yet/)
  assert.doesNotMatch(html, /<table\b/)
})

test("no search matches offers clearing", () => {
  const html = render(skills, "nothing")
  assert.match(html, /No skills match/)
  assert.match(html, /Clear search/)
  assert.doesNotMatch(html, /<table\b/)
})

test("names and descriptions render as text", () => {
  const html = render([{ name: "<script>x</script>", description: "<b>d</b>", problem: null }])
  assert.match(html, /&lt;script&gt;x&lt;\/script&gt;/)
  assert.match(html, /&lt;b&gt;d&lt;\/b&gt;/)
  assert.doesNotMatch(html, /<script\b|<b>/)
})
