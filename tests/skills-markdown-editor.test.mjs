import assert from "node:assert/strict"
import { after, before, test } from "node:test"
import { resolve } from "node:path"
import { createServer } from "vite"

let server
let mod
let cm

before(async () => {
  server = await createServer({
    configFile: false,
    resolve: { alias: { "@": resolve("src") } },
    esbuild: { jsx: "automatic" },
    server: { middlewareMode: true },
  })
  mod = await server.ssrLoadModule("/src/components/markdown-editor.tsx")
  cm = await server.ssrLoadModule("@codemirror/state")
})

after(async () => {
  await server.close()
})

/** Run a command on `doc` with selection [from, to); returns text and selection. */
const run = (name, doc, from, to = from) => {
  let state = cm.EditorState.create({
    doc,
    selection: cm.EditorSelection.single(from, to),
  })
  mod.markdownCommands[name]({ state, dispatch: (tr) => (state = tr.state) })
  const r = state.selection.main
  return [state.doc.toString(), state.sliceDoc(r.from, r.to)]
}

test("bold wraps and unwraps the selection", () => {
  assert.deepEqual(run("bold", "a word", 2, 6), ["a **word**", "word"])
  assert.deepEqual(run("bold", "a **word**", 4, 8), ["a word", "word"])
  assert.deepEqual(run("italic", "x", 1), ["x__", ""])
})

test("heading and list toggle a prefix on every selected line", () => {
  assert.deepEqual(run("list", "a\nb", 0, 3)[0], "- a\n- b")
  assert.deepEqual(run("list", "- a\n- b", 0, 7)[0], "a\nb")
  assert.deepEqual(run("heading", "title", 2)[0], "## title")
})

test("code is inline on one line and fenced across lines", () => {
  assert.deepEqual(run("code", "run x", 4, 5), ["run `x`", "x"])
  assert.deepEqual(run("code", "a\nb", 0, 3), ["```\na\nb\n```", "a\nb"])
})

test("link selects url when text given, text otherwise", () => {
  assert.deepEqual(run("link", "see docs", 4, 8), ["see [docs](url)", "url"])
  assert.deepEqual(run("link", "", 0), ["[text](url)", "text"])
})
