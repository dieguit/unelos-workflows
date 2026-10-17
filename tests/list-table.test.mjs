import assert from "node:assert/strict"
import { after, before, test } from "node:test"
import { resolve } from "node:path"
import { createServer } from "vite"
import { createElement as h } from "react"
import { renderToStaticMarkup } from "react-dom/server"
import { SearchXIcon } from "lucide-react"

let server
let kit
let search
let empty

before(async () => {
  server = await createServer({
    configFile: false,
    resolve: { alias: { "@": resolve("src") } },
    esbuild: { jsx: "automatic" },
    server: { middlewareMode: true },
  })
  kit = await server.ssrLoadModule("/src/components/list-table.tsx")
  search = await server.ssrLoadModule("/src/components/list-search.tsx")
  empty = await server.ssrLoadModule("/src/components/empty-state.tsx")
})

after(async () => {
  await server.close()
})

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
  assert.ok(th, "th with authored label")
  assert.match(th[0], /scope="col"/)
  for (const c of ["uppercase", "text-xs", "sticky", "-top-6", "w-70", "first:pl-3"])
    assert.match(th[0], new RegExp(`class="[^"]*(?<=[" ])${c.replace(/[:-]/g, "\\$&")}(?=[" ])`))
})

test("ListRow is focusable data-row with accent hover only", () => {
  const tr = table().match(/<tbody[^>]*>\s*<tr\b[^>]*>/)[0]
  assert.match(tr, /data-row="true"/)
  assert.match(tr, /tabindex="0"/)
  assert.match(tr, /hover:bg-accent/)
  assert.doesNotMatch(tr, /hover:bg-muted\/50/)
  assert.match(tr, /focus-visible:ring-2/)
})

test("ListCell pads first/last columns, ListPrimary title text-base", () => {
  const html = table()
  const td = html.match(/<td\b[^>]*>/)[0]
  for (const c of ["py-2.5", "first:pl-3", "last:pr-3"]) assert.ok(td.includes(c), c)
  assert.match(html, /class="[^"]*text-base[^"]*font-medium[^"]*">flow<\/span>/)
  assert.match(html, /class="[^"]*font-mono[^"]*text-xs[^"]*">flow\.yaml<\/span>/)
})

test("countLabel pluralizes by total and shows filtered form", () => {
  assert.equal(search.countLabel(2, 2, "agent", false), "2 agents")
  assert.equal(search.countLabel(1, 1, "workflow", false), "1 workflow")
  assert.equal(search.countLabel(1, 2, "agent", true), "1 of 2 agents")
  assert.equal(search.countLabel(2, 2, "workflow", true), "2 of 2 workflows")
  assert.equal(search.countLabel(0, 1, "agent", true), "0 of 1 agent")
})

test("ListSearch renders labelled search and polite live count", () => {
  const html = renderToStaticMarkup(
    h(search.ListSearch, { noun: "agent", value: " eng ", onChange() {}, shown: 1, total: 2, inputRef: { current: null } })
  )
  assert.match(html, /aria-label="Search agents"/)
  assert.match(html, /placeholder="Search agents"/)
  assert.match(html, /<span[^>]*data-list-count[^>]*aria-live="polite"[^>]*>1 of 2 agents<\/span>/)
  assert.match(html, /<kbd[^>]*>\/<\/kbd>/)
})

test("EmptyState dashed uses h2 title; plain uses p title, no border", () => {
  const dashed = renderToStaticMarkup(
    h(empty.EmptyState, { icon: SearchXIcon, title: "None yet", description: "Desc", dashed: true })
  )
  assert.match(dashed, /border-dashed/)
  assert.match(dashed, /<h2 class="text-lg font-semibold">None yet<\/h2>/)
  assert.match(dashed, /<p class="text-sm text-foreground-dim">Desc<\/p>/)
  const plain = renderToStaticMarkup(h(empty.EmptyState, { icon: SearchXIcon, title: "No match" }))
  assert.doesNotMatch(plain, /border/)
  assert.match(plain, /<p class="text-base">No match<\/p>/)
})

test("ListPrimary shows a focusable warning with the problem only when given", () => {
  const plain = renderToStaticMarkup(h(kit.ListPrimary, { title: "ok" }))
  assert.doesNotMatch(plain, /aria-label="Problem/)
  const html = renderToStaticMarkup(
    h(kit.ListPrimary, { title: "bad", problem: "model: <expected> a string" })
  )
  assert.match(html, />bad<\/span>/)
  assert.match(html, /aria-label="Problem: model: &lt;expected&gt; a string"/)
  assert.match(html, /<span[^>]*tabindex="0"[^>]*aria-label="Problem/)
  assert.match(html, /text-warning/)
})
