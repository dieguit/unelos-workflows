import * as kit from "./list-table"
import { expect, test } from "vitest"
import { createElement as h } from "react"
import { renderToStaticMarkup } from "react-dom/server"


const table = () =>
  renderToStaticMarkup(
    h(
      kit.ListTable,
      null,
      h(kit.ListHeader, null, h(kit.ListHead, { className: "w-70" }, "Name")),
      h(
        kit.ListBody,
        null,
        h(
          kit.ListRow,
          null,
          h(kit.ListCell, null, h(kit.ListPrimary, { title: "flow", subtitle: "flow.yaml" }))
        )
      )
    )
  )

test("ListHead uppercases via CSS, keeps authored label, sticky", () => {
  const th = table().match(/<th\b[^>]*>Name<\/th>/)
  expect(th, "th with authored label").toBeTruthy()
  expect(th![0]).toMatch(/scope="col"/)
  for (const c of ["uppercase", "text-xs", "sticky", "-top-6", "w-70", "first:pl-3"])
    expect(th![0]).toMatch(new RegExp(`class="[^"]*(?<=[" ])${c.replace(/[:-]/g, "\\$&")}(?=[" ])`))
})

test("ListRow is focusable data-row with accent hover only", () => {
  const tr = table().match(/<tbody[^>]*>\s*<tr\b[^>]*>/)![0]
  expect(tr).toMatch(/data-row="true"/)
  expect(tr).toMatch(/tabindex="0"/)
  expect(tr).toMatch(/hover:bg-accent/)
  expect(tr).not.toMatch(/hover:bg-muted\/50/)
  expect(tr).toMatch(/focus-visible:ring-2/)
})

test("ListCell pads first/last columns, ListPrimary title text-base", () => {
  const html = table()
  const td = html.match(/<td\b[^>]*>/)![0]
  for (const c of ["py-2.5", "first:pl-3", "last:pr-3"]) expect(td.includes(c), c).toBeTruthy()
  expect(html).toMatch(/class="[^"]*text-base[^"]*font-medium[^"]*">flow<\/span>/)
  expect(html).toMatch(/class="[^"]*font-mono[^"]*text-xs[^"]*">flow\.yaml<\/span>/)
})

test("ListPrimary shows a focusable warning with the problem only when given", () => {
  const plain = renderToStaticMarkup(h(kit.ListPrimary, { title: "ok" }))
  expect(plain).not.toMatch(/aria-label="Problem/)
  const html = renderToStaticMarkup(
    h(kit.ListPrimary, { title: "bad", problem: "model: <expected> a string" })
  )
  expect(html).toMatch(/>bad<\/span>/)
  expect(html).toMatch(/aria-label="Problem: model: &lt;expected&gt; a string"/)
  expect(html).toMatch(/<span[^>]*tabindex="0"[^>]*aria-label="Problem/)
  expect(html).toMatch(/text-warning/)
})
