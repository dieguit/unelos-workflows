import * as mod from "./skills-list"
import { expect, test } from "vitest"
import { createElement } from "react"
import { renderToStaticMarkup } from "react-dom/server"
import type { SkillSummary } from "@/lib/projects"


const skills = [
  { name: "code-review", description: "Review code for correctness.", problem: null },
  { name: "rust", description: null, problem: null },
]
const broken = { name: "bad", description: null, problem: "bad <yaml>" }
const render = (skills: SkillSummary[], query = "", selected?: string) =>
  renderToStaticMarkup(
    createElement(mod.SkillsList, { skills, query, onClear() {}, selected })
  )

test("filterSkills matches name or description, case-insensitive, trimmed", () => {
  const names = (q: string) => mod.filterSkills(skills, q).map((s) => s.name)
  expect(names(" RUST ")).toEqual(["rust"])
  expect(names("correctness")).toEqual(["code-review"])
  expect(names("nothing")).toEqual([])
})

test("table renders Name and Description with placeholder", () => {
  const html = render(skills)
  expect(html).toMatch(/class="[^"]*\buppercase\b[^"]*"[^>]*>Name<\/th>/)
  expect(html).toMatch(/>Description<\/th>/)
  expect(html).toMatch(/Review code for correctness\./)
  expect(html).toMatch(/—/)
  expect([...html.matchAll(/<tr\b[^>]*data-row[^>]*>/g)].length).toBe(2)
  expect(html).not.toMatch(/<a\b|New skill|<script\b|aria-label="Problem/)
})

test("broken skill is its own row with a warning, not a notice", () => {
  const html = render([broken, ...skills])
  expect([...html.matchAll(/<tr\b[^>]*data-row/g)].length).toBe(3)
  expect(html).toMatch(/aria-label="Problem: bad &lt;yaml&gt;"/)
  expect(html).not.toMatch(/with problems|<section\b/)
})

test("empty skills shows the no skills state", () => {
  const html = render([])
  expect(html).toMatch(/No skills yet/)
  expect(html).not.toMatch(/<table\b/)
})

test("no search matches offers clearing", () => {
  const html = render(skills, "nothing")
  expect(html).toMatch(/No skills match/)
  expect(html).toMatch(/Clear search/)
  expect(html).not.toMatch(/<table\b/)
})

test("names and descriptions render as text", () => {
  const html = render([{ name: "<script>x</script>", description: "<b>d</b>", problem: null }])
  expect(html).toMatch(/&lt;script&gt;x&lt;\/script&gt;/)
  expect(html).toMatch(/&lt;b&gt;d&lt;\/b&gt;/)
  expect(html).not.toMatch(/<script\b|<b>/)
})

test("selected skill row is marked selected", () => {
  const html = render(skills, "", "rust")
  expect([...html.matchAll(/aria-selected="true"/g)].length).toBe(1)
  expect(html).toMatch(/aria-selected="true"[^>]*>(?:(?!<tr).)*rust/s)
})
