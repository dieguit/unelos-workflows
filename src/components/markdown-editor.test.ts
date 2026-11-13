import * as cm from "@codemirror/state"
import * as mod from "@/components/markdown-editor"
import { expect, test } from "vitest"


/** Run a command on `doc` with selection [from, to); returns text and selection. */
const run = (name: keyof typeof mod.markdownCommands, doc: string, from: number, to = from) => {
  let state = cm.EditorState.create({
    doc,
    selection: cm.EditorSelection.single(from, to),
  })
  mod.markdownCommands[name]({ state, dispatch: (tr) => (state = tr.state) })
  const r = state.selection.main
  return [state.doc.toString(), state.sliceDoc(r.from, r.to)]
}

test("bold wraps and unwraps the selection", () => {
  expect(run("bold", "a word", 2, 6)).toEqual(["a **word**", "word"])
  expect(run("bold", "a **word**", 4, 8)).toEqual(["a word", "word"])
  expect(run("italic", "x", 1)).toEqual(["x__", ""])
})

test("heading and list toggle a prefix on every selected line", () => {
  expect(run("list", "a\nb", 0, 3)[0]).toEqual("- a\n- b")
  expect(run("list", "- a\n- b", 0, 7)[0]).toEqual("a\nb")
  expect(run("heading", "title", 2)[0]).toEqual("## title")
})

test("code is inline on one line and fenced across lines", () => {
  expect(run("code", "run x", 4, 5)).toEqual(["run `x`", "x"])
  expect(run("code", "a\nb", 0, 3)).toEqual(["```\na\nb\n```", "a\nb"])
})

test("link selects url when text given, text otherwise", () => {
  expect(run("link", "see docs", 4, 8)).toEqual(["see [docs](url)", "url"])
  expect(run("link", "", 0)).toEqual(["[text](url)", "text"])
})
