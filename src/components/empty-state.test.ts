import { expect, test } from "vitest"
import { createElement as h } from "react"
import { renderToStaticMarkup } from "react-dom/server"
import * as empty from "./empty-state"
import { SearchXIcon } from "lucide-react"

test("EmptyState dashed uses h2 title; plain uses p title, no border", () => {
  const dashed = renderToStaticMarkup(
    h(empty.EmptyState, { icon: SearchXIcon, title: "None yet", description: "Desc", dashed: true })
  )
  expect(dashed).toMatch(/border-dashed/)
  expect(dashed).toMatch(/<h2 class="text-lg font-semibold">None yet<\/h2>/)
  expect(dashed).toMatch(/<p class="text-sm text-foreground-dim">Desc<\/p>/)
  const plain = renderToStaticMarkup(h(empty.EmptyState, { icon: SearchXIcon, title: "No match" }))
  expect(plain).not.toMatch(/border/)
  expect(plain).toMatch(/<p class="text-base">No match<\/p>/)
})
