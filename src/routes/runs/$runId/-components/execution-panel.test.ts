import { createElement as h } from "react"
import { renderToStaticMarkup } from "react-dom/server"
import { expect, test } from "vitest"
import { ExecutionPanel } from "./execution-panel"

test("execution list shows full child node ids", () => {
  const html = renderToStaticMarkup(h(ExecutionPanel, {
    nodeId: "frontend", kind: "task", onOpen: () => {}, onClose: () => {},
    executions: [{
      n: 1, active: false, outcome: "done", reason: "ready",
      record: { nodeId: "frontend/design", executionId: "e1", inputs: [], outputs: [], workerReports: [], feedback: null },
    }],
  }))
  expect(html).toContain("frontend/design")
  expect(html).toContain("done")
  expect(html).toContain("ready")
})
