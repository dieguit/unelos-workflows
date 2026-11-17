import { WorkflowsTable } from "./workflows-table"
import { expect, test } from "vitest"
import { createElement } from "react"
import { renderToStaticMarkup } from "react-dom/server"

test("workflow rows expose a separate actions control", async () => {
  const html = renderToStaticMarkup(
      createElement(WorkflowsTable, {
        workflows: [
          {
            name: "Example",
            calls: [],
            file: "example.yaml",
            description: null,
            steps: 1,
            problem: null,
          },
        ],
        onOpen: () => {},
        actions: () =>
          createElement("button", { "aria-label": "Actions for Example" }),
      })
    )
    const buttons = [...html.matchAll(/<button\b([^>]*)>(.*?)<\/button>/gs)]
    expect(buttons.length).toBe(1)
    expect(buttons[0][1]).toMatch(/aria-label="Actions for Example"/)
    expect(html).toMatch(/<tr[^>]*tabindex="0"/)
})

test("unreadable workflow shows a warning by its name and — for unknown cells", async () => {
  const html = renderToStaticMarkup(
      createElement(WorkflowsTable, {
        workflows: [
          {
            name: "bad",
            calls: [],
            file: "bad.yaml",
            description: null,
            steps: null,
            problem: "parse error",
          },
        ],
        actions: () => null,
        onOpen: () => {},
      })
    )
    expect(html).toMatch(/aria-label="Problem: parse error"/)
    const cells = [...html.matchAll(/<td\b[^>]*>(.*?)<\/td>/gs)].map(
      (m) => m[1]
    )
    expect(cells[1]).toBe("—")
    expect(cells[2]).toBe("—")
    expect(html).not.toMatch(/>Definition<|Valid/)
})
