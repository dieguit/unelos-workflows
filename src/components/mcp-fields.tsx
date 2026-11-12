import { PlusIcon, XIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import type { McpDocument } from "@/lib/projects"

type Pairs = [string, string][]

function PairsEditor({
  label,
  hint,
  pairs,
  onChange,
}: {
  label: string
  hint?: string
  pairs: Pairs
  onChange: (next: Pairs) => void
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-xs font-medium text-foreground-dim">{label}</span>
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
      {pairs.map(([k, v], i) => (
        <div key={i} className="flex gap-1.5">
          <Input
            aria-label={`${label} name`}
            value={k}
            placeholder="NAME"
            spellCheck={false}
            className="h-8 w-2/5 font-mono text-xs"
            onChange={(e) =>
              onChange(pairs.map((p, j) => (j === i ? [e.target.value, v] : p)))
            }
          />
          <Input
            aria-label={`${label} value`}
            value={v}
            placeholder="{env:VAR_NAME}"
            spellCheck={false}
            className="h-8 min-w-0 flex-1 font-mono text-xs"
            onChange={(e) =>
              onChange(pairs.map((p, j) => (j === i ? [k, e.target.value] : p)))
            }
          />
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={`Remove ${label} ${k}`}
            onClick={() => onChange(pairs.filter((_, j) => j !== i))}
          >
            <XIcon aria-hidden="true" />
          </Button>
        </div>
      ))}
      <Button
        variant="outline"
        size="sm"
        className="self-start"
        onClick={() => onChange([...pairs, ["", ""]])}
      >
        <PlusIcon aria-hidden="true" />
        Add
      </Button>
    </div>
  )
}

const toRecord = (pairs: Pairs) =>
  Object.fromEntries(
    pairs.filter(([k]) => k.trim()).map(([k, v]) => [k.trim(), v])
  )

export type McpDraft = {
  id: string
  type: "local" | "remote"
  command: string
  env: Pairs
  url: string
  headers: Pairs
}
export const emptyMcp: McpDraft = {
  id: "",
  type: "local",
  command: "",
  env: [],
  url: "",
  headers: [],
}
export function mcpDraft(m: McpDocument): McpDraft {
  return {
    ...emptyMcp,
    id: m.id,
    type: m.type,
    ...(m.type === "local"
      ? { command: m.command.join(" "), env: Object.entries(m.environment) }
      : { url: m.url, headers: Object.entries(m.headers) }),
  }
}
export function mcpDocument(draft: McpDraft): McpDocument {
  // shortcut: command split on whitespace, no quoting; upgrade if args with spaces are needed
  return draft.type === "local"
    ? {
        id: draft.id.trim(),
        type: draft.type,
        command: draft.command.trim().split(/\s+/).filter(Boolean),
        environment: toRecord(draft.env),
      }
    : {
        id: draft.id.trim(),
        type: draft.type,
        url: draft.url.trim(),
        headers: toRecord(draft.headers),
      }
}
export function McpFields({
  draft,
  onChange,
}: {
  draft: McpDraft
  onChange: (draft: McpDraft) => void
}) {
  const { id, type, command, env, url, headers } = draft
  const update = (next: Partial<McpDraft>) => onChange({ ...draft, ...next })
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-1.5">
        <label
          htmlFor="mcp-id"
          className="text-xs font-medium text-foreground-dim"
        >
          Name (id)
        </label>
        <Input
          id="mcp-id"
          value={id}
          autoFocus
          spellCheck={false}
          autoComplete="off"
          placeholder="e.g. github"
          className="h-9 font-mono"
          onChange={(e) => update({ id: e.target.value })}
        />
      </div>
      <div role="group" aria-label="Type" className="flex gap-1.5">
        {(["local", "remote"] as const).map((t) => (
          <Button
            key={t}
            size="sm"
            variant={type === t ? "secondary" : "outline"}
            aria-pressed={type === t}
            onClick={() => update({ type: t })}
          >
            {t === "local" ? "Local" : "Remote"}
          </Button>
        ))}
      </div>
      {type === "local" ? (
        <>
          <div className="flex flex-col gap-1.5">
            <label
              htmlFor="mcp-command"
              className="text-xs font-medium text-foreground-dim"
            >
              Command (split on spaces)
            </label>
            <Input
              id="mcp-command"
              value={command}
              spellCheck={false}
              placeholder="npx -y @modelcontextprotocol/server-github"
              className="h-9 font-mono text-xs"
              onChange={(e) => update({ command: e.target.value })}
            />
          </div>
          <PairsEditor
            label="Environment"
            hint="Prefer {env:VAR_NAME} over literal values, so secrets stay in your OS environment instead of project files."
            pairs={env}
            onChange={(env) => update({ env })}
          />
        </>
      ) : (
        <>
          <div className="flex flex-col gap-1.5">
            <label
              htmlFor="mcp-url"
              className="text-xs font-medium text-foreground-dim"
            >
              URL
            </label>
            <Input
              id="mcp-url"
              value={url}
              spellCheck={false}
              placeholder="https://example.com/mcp"
              className="h-9 font-mono text-xs"
              onChange={(e) => update({ url: e.target.value })}
            />
          </div>
          <PairsEditor
            label="Headers"
            pairs={headers}
            onChange={(headers) => update({ headers })}
          />
        </>
      )}
    </div>
  )
}
