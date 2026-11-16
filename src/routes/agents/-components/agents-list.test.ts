import * as mod from "@/routes/agents/-components/agents-list"
import { expect, test } from "vitest"
import { createElement } from "react"
import { renderToStaticMarkup } from "react-dom/server"
import type { AgentSummary } from "@/lib/projects"


const agents = [
  { name: "engineer", model: "provider/fast", skills: ["rust"], mcps: ["github"], problem: null },
  { name: "reviewer", model: null, skills: [], mcps: [], problem: null },
]
const broken = { name: "broken", model: null, skills: [], mcps: [], problem: "bad <yaml>" }
const render = (agents: AgentSummary[], query = "") =>
  renderToStaticMarkup(
    createElement(mod.AgentsList, {
      agents,
      query,
      onClear() {},
      usedBy: { of: (id) => (id === "engineer" ? [{ kind: "workflow", id: "w", name: "w" }] : []) },
    })
  )

test("used-by column shows workflow count", () => {
  const html = render(agents)
  expect(html).toMatch(/>Used by<\/th>/)
  expect(html).toMatch(/>1 workflow<\/td>/)
})

test("filterAgents matches names only, case-insensitive, trimmed", () => {
  expect(mod.filterAgents(agents, " ENGINE ").map((a) => a.name)).toEqual(["engineer"])
  for (const query of ["provider/fast", "rust", "github"])
    expect(mod.filterAgents(agents, query)).toEqual([])
})

test("table renders columns, placeholders and no interactive chrome", () => {
  const html = render(agents)
  for (const heading of ["Name", "Model", "Skills", "MCPs"])
    expect(html).toMatch(new RegExp(heading))
  expect(html).toMatch(/Not configured/)
  expect(html).toMatch(/—/)
  const rows = [...html.matchAll(/<tr\b[^>]*data-row[^>]*>/g)]
  expect(rows.length).toBe(2)
  for (const [tr] of rows) expect(tr).toMatch(/tabindex="0"/)
  expect(html).toMatch(/class="[^"]*text-base[^"]*font-medium[^"]*">engineer<\/span>/)
  expect(html).toMatch(/class="[^"]*\buppercase\b[^"]*"[^>]*>Name<\/th>/)
  expect(html).not.toMatch(/<a\b|New Agent|<script\b|aria-label="Problem/)
})

test("broken agent is its own row with a warning, not a notice", () => {
  const html = render([broken, ...agents])
  expect([...html.matchAll(/<tr\b[^>]*data-row/g)].length).toBe(3)
  expect(html).toMatch(/aria-label="Problem: bad &lt;yaml&gt;"/)
  expect([...html.matchAll(/Not configured/g)].length, "broken row shows — not Not configured").toBe(1)
  expect(html).not.toMatch(/with problems|<section\b/)
})

test("empty agents shows the no agents state", () => {
  const html = render([])
  expect(html).toMatch(/No agents yet/)
  expect(html).not.toMatch(/<table\b/)
})

test("no search matches offers clearing", () => {
  const html = render(agents, "nothing")
  expect(html).toMatch(/No agents match/)
  expect(html).toMatch(/Clear search/)
  expect(html).not.toMatch(/<table\b/)
})

test("names and references render as text, long names stay non-links", () => {
  const name = "<script>x</script>" + "a".repeat(200)
  const html = render([
    { name, model: "<b>m</b>", skills: ["<i>s</i>"], mcps: ["<u>g</u>"], problem: null },
  ])
  expect(html).toMatch(/&lt;script&gt;x&lt;\/script&gt;a{200}/)
  expect(html).toMatch(/&lt;b&gt;m&lt;\/b&gt;/)
  expect(html).not.toMatch(/<a\b|<script\b|<b>|<i>|<u>/)
})
