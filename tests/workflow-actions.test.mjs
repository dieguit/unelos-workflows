import assert from "node:assert/strict"
import { test } from "node:test"
import { resolve } from "node:path"
import { createServer } from "vite"
import { createElement } from "react"
import { renderToStaticMarkup } from "react-dom/server"

test("workflow rows expose a separate actions control", async () => {
  const server = await createServer({
    configFile: false,
    resolve: { alias: { "@": resolve("src") } },
    esbuild: { jsx: "automatic" },
    server: { middlewareMode: true },
  })
  try {
    const { WorkflowsTable } = await server.ssrLoadModule(
      "/src/routes/workflows/-components/workflows-table.tsx"
    )
    const html = renderToStaticMarkup(
      createElement(WorkflowsTable, {
        workflows: [
          {
            name: "Example",
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
    assert.equal(buttons.length, 1)
    assert.match(buttons[0][1], /aria-label="Actions for Example"/)
    assert.match(html, /<tr[^>]*tabindex="0"/)
  } finally {
    await server.close()
  }
})

test("unreadable workflow shows a warning by its name and — for unknown cells", async () => {
  const server = await createServer({
    configFile: false,
    resolve: { alias: { "@": resolve("src") } },
    esbuild: { jsx: "automatic" },
    server: { middlewareMode: true },
  })
  try {
    const { WorkflowsTable } = await server.ssrLoadModule(
      "/src/routes/workflows/-components/workflows-table.tsx"
    )
    const html = renderToStaticMarkup(
      createElement(WorkflowsTable, {
        workflows: [
          {
            name: "bad",
            file: "bad.yaml",
            description: null,
            steps: null,
            problem: "parse error",
          },
        ],
        actions: () => null,
      })
    )
    assert.match(html, /aria-label="Problem: parse error"/)
    const cells = [...html.matchAll(/<td\b[^>]*>(.*?)<\/td>/gs)].map(
      (m) => m[1]
    )
    assert.equal(cells[1], "—")
    assert.equal(cells[2], "—")
    assert.doesNotMatch(html, />Definition<|Valid/)
  } finally {
    await server.close()
  }
})
