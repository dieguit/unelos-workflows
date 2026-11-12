import { expect, test } from "bun:test"
import { emptyMcp, mcpDocument, mcpDraft } from "./mcp-fields"

test("local MCP fields round-trip and trim names without altering secrets", () => {
  const document = {
    id: "github",
    type: "local",
    command: ["npx", "-y", "server"],
    environment: { TOKEN: "{env:TOKEN}" },
  }
  expect(mcpDocument(mcpDraft(document))).toEqual(document)
  expect(
    mcpDocument({
      ...emptyMcp,
      id: " local ",
      command: " npx  server ",
      env: [
        [" TOKEN ", " secret "],
        ["", "ignored"],
      ],
    })
  ).toEqual({
    id: "local",
    type: "local",
    command: ["npx", "server"],
    environment: { TOKEN: " secret " },
  })
})

test("remote MCP fields round-trip independently of local fields", () => {
  const document = {
    id: "remote",
    type: "remote",
    url: "https://example.com/mcp",
    headers: { Authorization: "{env:TOKEN}" },
  }
  expect(mcpDocument(mcpDraft(document))).toEqual(document)
})
