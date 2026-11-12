import { useEffect, useRef, useState } from "react"
import Markdown from "react-markdown"
import remarkGfm from "remark-gfm"
import {
  EditorSelection,
  EditorState,
  type StateCommand,
} from "@codemirror/state"
import {
  EditorView,
  keymap,
  placeholder as cmPlaceholder,
} from "@codemirror/view"
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands"
import { markdown } from "@codemirror/lang-markdown"
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language"
import { tags } from "@lezer/highlight"
import {
  BoldIcon,
  CodeIcon,
  EyeIcon,
  HeadingIcon,
  ItalicIcon,
  LinkIcon,
  ListIcon,
  type LucideIcon,
  PencilIcon,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

/** Wrap each selection in `mark`, or unwrap when already wrapped. */
const wrap =
  (mark: string): StateCommand =>
  ({ state, dispatch }) => {
    const n = mark.length
    dispatch(
      state.update(
        state.changeByRange((r) => {
          const before = state.sliceDoc(r.from - n, r.from)
          const after = state.sliceDoc(r.to, r.to + n)
          if (before === mark && after === mark)
            return {
              changes: [
                { from: r.from - n, to: r.from },
                { from: r.to, to: r.to + n },
              ],
              range: EditorSelection.range(r.from - n, r.to - n),
            }
          return {
            changes: [
              { from: r.from, insert: mark },
              { from: r.to, insert: mark },
            ],
            range: EditorSelection.range(r.from + n, r.to + n),
          }
        }),
        { scrollIntoView: true, userEvent: "input" }
      )
    )
    return true
  }

/** Toggle `prefix` on every line touched by the selection. */
const linePrefix =
  (prefix: string): StateCommand =>
  ({ state, dispatch }) => {
    const lines = new Set<number>()
    for (const r of state.selection.ranges)
      for (
        let l = state.doc.lineAt(r.from).number;
        l <= state.doc.lineAt(r.to).number;
        l++
      )
        lines.add(l)
    const all = [...lines].map((l) => state.doc.line(l))
    const remove = all.every((line) => line.text.startsWith(prefix))
    const changes = all.map((line) =>
      remove
        ? { from: line.from, to: line.from + prefix.length }
        : { from: line.from, insert: prefix }
    )
    dispatch(
      state.update({ changes, scrollIntoView: true, userEvent: "input" })
    )
    return true
  }

const code: StateCommand = (target) => {
  const { state, dispatch } = target
  const multi = state.selection.ranges.some(
    (r) => state.doc.lineAt(r.from).number !== state.doc.lineAt(r.to).number
  )
  if (!multi) return wrap("`")(target)
  dispatch(
    state.update(
      state.changeByRange((r) => {
        const from = state.doc.lineAt(r.from).from
        const to = state.doc.lineAt(r.to).to
        return {
          changes: [
            { from, insert: "```\n" },
            { from: to, insert: "\n```" },
          ],
          range: EditorSelection.range(from + 4, to + 4),
        }
      }),
      { scrollIntoView: true, userEvent: "input" }
    )
  )
  return true
}

/** `[text](url)` with "url" selected, or "text" selected when nothing was. */
const link: StateCommand = ({ state, dispatch }) => {
  dispatch(
    state.update(
      state.changeByRange((r) => {
        const text = state.sliceDoc(r.from, r.to) || "text"
        const insert = `[${text}](url)`
        const range = r.empty
          ? EditorSelection.range(r.from + 1, r.from + 1 + text.length)
          : EditorSelection.range(
              r.from + text.length + 3,
              r.from + text.length + 6
            )
        return { changes: { from: r.from, to: r.to, insert }, range }
      }),
      { scrollIntoView: true, userEvent: "input" }
    )
  )
  return true
}

/** Exported for tests. */
export const markdownCommands = {
  bold: wrap("**"),
  italic: wrap("_"),
  heading: linePrefix("## "),
  list: linePrefix("- "),
  code,
  link,
}

const actions: {
  label: string
  icon: LucideIcon
  run: StateCommand
  keys?: string
}[] = [
  { label: "Bold", icon: BoldIcon, run: markdownCommands.bold, keys: "Mod-b" },
  {
    label: "Italic",
    icon: ItalicIcon,
    run: markdownCommands.italic,
    keys: "Mod-i",
  },
  { label: "Heading", icon: HeadingIcon, run: markdownCommands.heading },
  { label: "List", icon: ListIcon, run: markdownCommands.list },
  { label: "Code", icon: CodeIcon, run: markdownCommands.code },
  { label: "Link", icon: LinkIcon, run: markdownCommands.link, keys: "Mod-k" },
]

const highlight = HighlightStyle.define([
  { tag: tags.heading, fontWeight: "700", color: "var(--foreground)" },
  { tag: tags.strong, fontWeight: "700" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: [tags.link, tags.url], color: "var(--primary)" },
  { tag: tags.monospace, color: "var(--primary)" },
  {
    tag: [tags.processingInstruction, tags.list, tags.quote],
    color: "var(--muted-foreground)",
  },
])

const theme = EditorView.theme({
  "&": {
    height: "100%",
    color: "var(--foreground)",
    backgroundColor: "transparent",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": {
    fontFamily: "var(--font-mono)",
    fontSize: "13px",
    lineHeight: "1.7",
    overflow: "auto",
  },
  ".cm-content": { padding: "16px", caretColor: "var(--primary)" },
  ".cm-line": { padding: "0" },
  ".cm-cursor": { borderLeftColor: "var(--primary)" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection":
    {
      backgroundColor: "var(--primary-muted) !important",
    },
  ".cm-placeholder": { color: "var(--muted-foreground)" },
})

export function MarkdownEditor({
  value,
  onChange,
  className,
  autoFocus,
  placeholder,
  readOnly,
  "aria-label": ariaLabel,
}: {
  value: string
  onChange?: (value: string) => void
  readOnly?: boolean
  className?: string
  autoFocus?: boolean
  placeholder?: string
  "aria-label": string
}) {
  const [preview, setPreview] = useState(!!readOnly)
  const host = useRef<HTMLDivElement>(null)
  const view = useRef<EditorView>(null)
  const latest = useRef({ onChange })
  useEffect(() => {
    latest.current.onChange = onChange
  })

  // Created once; later `value` changes are synced by the effect below.
  useEffect(() => {
    const v = new EditorView({
      parent: host.current!,
      state: EditorState.create({
        doc: value,
        extensions: [
          history(),
          keymap.of([
            ...actions.flatMap((a) =>
              a.keys ? [{ key: a.keys, run: a.run }] : []
            ),
            ...historyKeymap,
            ...defaultKeymap,
          ]),
          markdown(),
          syntaxHighlighting(highlight),
          EditorView.lineWrapping,
          EditorView.contentAttributes.of({
            "aria-label": ariaLabel,
            "aria-multiline": "true",
          }),
          placeholder ? cmPlaceholder(placeholder) : [],
          EditorState.readOnly.of(!!readOnly),
          EditorView.editable.of(!readOnly),
          theme,
          EditorView.updateListener.of((u) => {
            if (u.docChanged) latest.current.onChange?.(u.state.doc.toString())
          }),
        ],
      }),
    })
    view.current = v
    if (autoFocus) v.focus()
    return () => v.destroy()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  useEffect(() => {
    const v = view.current
    if (!v || v.state.doc.toString() === value) return
    v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert: value } })
  }, [value])

  return (
    <div
      className={cn(
        "flex min-h-0 flex-col overflow-hidden rounded-lg border bg-card",
        className
      )}
    >
      <div
        role="toolbar"
        aria-label="Formatting"
        className="flex flex-wrap items-center gap-1 border-b px-2 py-1.5"
      >
        {!readOnly &&
          actions.map(({ label, icon: Icon, run, keys }) => (
            <Button
              key={label}
              type="button"
              variant="ghost"
              size="sm"
              disabled={preview}
              title={keys ? `${label} (${keys.replace("Mod", "Ctrl")})` : label}
              // keep the editor's selection/focus when clicking
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => {
                const v = view.current
                if (!v) return
                run(v)
                v.focus()
              }}
            >
              <Icon aria-hidden="true" />
              {label}
            </Button>
          ))}
        {!readOnly && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="ml-auto"
            aria-pressed={preview}
            onClick={() => {
              setPreview(!preview)
              if (preview) requestAnimationFrame(() => view.current?.focus())
            }}
          >
            {preview ? (
              <PencilIcon aria-hidden="true" />
            ) : (
              <EyeIcon aria-hidden="true" />
            )}
            {preview ? "Edit" : "Preview"}
          </Button>
        )}
      </div>
      {/* hidden, not unmounted, so undo history and selection survive */}
      <div ref={host} hidden={preview} className="min-h-0 flex-1" />
      {preview && (
        <div className="min-h-0 flex-1 overflow-auto p-4 text-sm leading-relaxed [&_a]:text-primary [&_a]:underline [&_blockquote]:border-l-2 [&_blockquote]:pl-3 [&_blockquote]:text-muted-foreground [&_code]:font-mono [&_code]:text-primary [&_h1]:text-xl [&_h1]:font-semibold [&_h2]:text-lg [&_h2]:font-semibold [&_h3]:font-semibold [&_ol]:list-decimal [&_ol]:pl-6 [&_pre]:overflow-auto [&_pre]:rounded-md [&_pre]:bg-muted [&_pre]:p-3 [&_table]:border-collapse [&_td]:border [&_td]:px-2 [&_th]:border [&_th]:px-2 [&_ul]:list-disc [&_ul]:pl-6 [&>*+*]:mt-3">
          <Markdown remarkPlugins={[remarkGfm]}>{value}</Markdown>
        </div>
      )}
    </div>
  )
}
