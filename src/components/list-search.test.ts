import { expect, test } from "vitest"
import { createElement as h } from "react"
import { renderToStaticMarkup } from "react-dom/server"
import * as search from "./list-search"

test("countLabel pluralizes by total and shows filtered form", () => {
  expect(search.countLabel(2, 2, "agent", false)).toBe("2 agents")
  expect(search.countLabel(1, 1, "workflow", false)).toBe("1 workflow")
  expect(search.countLabel(1, 2, "agent", true)).toBe("1 of 2 agents")
  expect(search.countLabel(2, 2, "workflow", true)).toBe("2 of 2 workflows")
  expect(search.countLabel(0, 1, "agent", true)).toBe("0 of 1 agent")
})

test("ListSearch renders labelled search and polite live count", () => {
  const html = renderToStaticMarkup(
    h(search.ListSearch, { noun: "agent", value: " eng ", onChange() {}, shown: 1, total: 2, inputRef: { current: null } })
  )
  expect(html).toMatch(/aria-label="Search agents"/)
  expect(html).toMatch(/placeholder="Search agents"/)
  expect(html).toMatch(/<span[^>]*data-list-count[^>]*aria-live="polite"[^>]*>1 of 2 agents<\/span>/)
  expect(html).toMatch(/<kbd[^>]*>\/<\/kbd>/)
})

