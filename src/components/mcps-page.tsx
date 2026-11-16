import { useEffect, useRef, useState, type ReactNode } from "react"
import { useBlocker, useRouter } from "@tanstack/react-router"
import {
  CableIcon,
  PanelRightCloseIcon,
  PlusIcon,
  SearchXIcon,
} from "lucide-react"
import { z } from "zod"
import { Button } from "@/components/ui/button"
import { BusyContent } from "@/components/busy-content"
import { EmptyState } from "@/components/empty-state"
import { ListActions } from "@/components/list-actions"
import { ListSearch } from "@/components/list-search"
import {
  ListBody,
  ListCell,
  ListHead,
  ListHeader,
  ListPrimary,
  ListRow,
  ListTable,
  Truncated,
} from "@/components/list-table"
import {
  emptyMcp,
  McpFields,
  mcpDocument,
  mcpDraft,
} from "@/components/mcp-fields"
import { useDiscardChanges } from "@/hooks/use-discard-changes"
import { useProjectDiscardGuard } from "@/hooks/use-project-discard-guard"
import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import { UsedByCell, UsedByHead, UsedByRow } from "@/components/used-by"
import type { McpDocument, McpSummary } from "@/lib/projects"
import type { UsedBy } from "@/lib/references"

type McpsApi = {
  get: (id: string) => Promise<McpDocument>
  save: (originalId: string | null, mcp: McpDocument) => Promise<McpDocument>
  remove: (id: string) => Promise<void>
  /** Project scope only. */
  makeGlobal?: (id: string) => Promise<void>
}

export function McpsPage({
  mcps,
  usedBy,
  initial,
  api,
  scope,
  header: Header,
}: {
  mcps: McpSummary[]
  usedBy: UsedBy
  initial?: string
  api: McpsApi
  scope: { path: string; label: string }
  header: (props: { action?: ReactNode }) => ReactNode
}) {
  const router = useRouter()
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)
  const [selected, setSelected] = useState<string | null | undefined>(initial)
  const [draft, setDraft] = useState(emptyMcp)
  const [baseline, setBaseline] = useState(emptyMcp)
  const [loaded, setLoaded] = useState(false)
  const [error, setError] = useState<string>()
  const [saving, setSaving] = useState(false)
  const visualSaving = useDelayedFlag(saving)
  const request = useRef(0)
  const dirty = loaded && JSON.stringify(draft) !== JSON.stringify(baseline)
  const discard = useDiscardChanges()
  const approved = useProjectDiscardGuard(saving, dirty, discard.ask)
  useBlocker({
    shouldBlockFn: async () =>
      !approved.current && (saving || (dirty && !(await discard.ask()))),
    enableBeforeUnload: dirty || saving,
  })
  const load = (id: string) => {
    const token = ++request.current
    api.get(id).then(
      (mcp) => {
        if (token !== request.current) return
        const next = mcpDraft(mcp)
        setDraft(next)
        setBaseline(next)
        setLoaded(true)
      },
      (err) => {
        if (token === request.current) setError(String(err))
      }
    )
  }
  useEffect(() => {
    if (initial) load(initial)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])
  const select = async (id: string | null | undefined) => {
    if (saving || id === selected || (dirty && !(await discard.ask()))) return
    ++request.current
    setSelected(id)
    setError(undefined)
    setDraft(emptyMcp)
    setBaseline(emptyMcp)
    setLoaded(id === null)
    if (typeof id === "string") load(id)
  }
  const save = async () => {
    setSaving(true)
    setError(undefined)
    try {
      const saved = await api.save(selected ?? null, mcpDocument(draft))
      const next = mcpDraft(saved)
      setDraft(next)
      setBaseline(next)
      setSelected(saved.id)
      void router.invalidate()
    } catch (err) {
      setError(String(err))
    } finally {
      setSaving(false)
    }
  }
  const shown = mcps.filter((m) =>
    m.name.toLowerCase().includes(query.trim().toLowerCase())
  )
  const closeIf = (name: string) => {
    if (selected !== name) return
    ++request.current
    setSelected(undefined)
    setLoaded(false)
    setDraft(emptyMcp)
    setBaseline(emptyMcp)
  }
  const actions = (name: string, variant?: "footer") => (
    <ListActions
      variant={variant}
      kind="MCP"
      name={name}
      nameSchema={z.string()}
      disabled={visualSaving}
      beforeRemove={async () =>
        selected !== name || (!saving && (!dirty || (await discard.ask())))
      }
      remove={async () => {
        await api.remove(name)
        closeIf(name)
        void router.invalidate()
      }}
      makeGlobal={
        api.makeGlobal &&
        (async () => {
          await api.makeGlobal!(name)
          closeIf(name)
          void router.invalidate()
        })
      }
      openGlobal={() =>
        void router.navigate({ to: "/global-mcps", search: { mcp: name } })
      }
    />
  )
  return (
    <div
      inert={saving}
      className="-mx-8 -mt-6 -mb-8 flex h-[calc(100%+3.5rem)]"
    >
      <div className="min-w-0 flex-1 overflow-y-auto px-8 pt-6 pb-8">
        <div className="flex flex-col gap-4">
          <Header
            action={
              <Button onClick={() => void select(null)} disabled={visualSaving}>
                <PlusIcon aria-hidden="true" />
                New MCP
              </Button>
            }
          />
          {mcps.length > 0 && (
            <ListSearch
              noun="MCP"
              value={query}
              onChange={setQuery}
              shown={shown.length}
              total={mcps.length}
              inputRef={search}
            />
          )}
          {!mcps.length ? (
            <EmptyState
              dashed
              icon={CableIcon}
              title="No MCPs yet"
              description="Configure MCP servers for your agents."
              action={
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => void select(null)}
                >
                  <PlusIcon aria-hidden="true" />
                  New MCP
                </Button>
              }
            />
          ) : !shown.length ? (
            <EmptyState
              icon={SearchXIcon}
              title={`No MCPs match “${query.trim()}”`}
              action={
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setQuery("")
                    search.current?.focus()
                  }}
                >
                  Clear search
                </Button>
              }
            />
          ) : (
            <ListTable>
              <ListHeader>
                <ListHead className="w-[26%]">Name</ListHead>
                <ListHead>Type</ListHead>
                <ListHead>Target</ListHead>
                <UsedByHead className="w-28" project={usedBy.project} />
                <ListHead className="w-14">
                  <span className="sr-only">Actions</span>
                </ListHead>
              </ListHeader>
              <ListBody>
                {shown.map((m) => (
                  <ListRow
                    key={m.name}
                    aria-selected={selected === m.name}
                    className="cursor-pointer aria-selected:bg-primary-muted"
                    onClick={() => void select(m.name)}
                    onKeyDown={(e) => {
                      if (
                        e.target !== e.currentTarget ||
                        (e.key !== "Enter" && e.key !== " ")
                      )
                        return
                      e.preventDefault()
                      void select(m.name)
                    }}
                  >
                    <ListCell>
                      <ListPrimary title={m.name} problem={m.problem} />
                    </ListCell>
                    <ListCell className="text-foreground-dim">
                      {m.kind ?? "—"}
                    </ListCell>
                    <ListCell className="font-mono text-xs text-foreground-dim">
                      {m.target ? <Truncated text={m.target} /> : "—"}
                    </ListCell>
                    <UsedByCell refs={usedBy.of(m.name)} />
                    <ListCell>
                      {actions(m.name)}
                    </ListCell>
                  </ListRow>
                ))}
              </ListBody>
            </ListTable>
          )}
        </div>
      </div>
      {selected !== undefined && (
        <aside
          aria-label={selected ? `MCP ${selected}` : "New MCP"}
          className="flex w-[380px] max-w-full shrink-0 flex-col border-l bg-background"
        >
          <header className="flex min-h-16 shrink-0 items-center gap-2 px-4 py-3">
            <div className="min-w-0 flex-1">
              <h2 className="truncate text-base font-semibold">
                {selected ?? "New MCP"}
              </h2>
              <p className="truncate font-mono text-xs text-muted-foreground">
                {selected ? `${scope.path}/${selected}` : scope.label}
              </p>
              {dirty && (
                <p role="status" className="mt-1 text-xs text-warning">
                  Unsaved changes
                </p>
              )}
            </div>
            <Button
              disabled={!dirty || visualSaving}
              onClick={() => void save()}
            >
              <BusyContent busy={visualSaving}>Save</BusyContent>
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Close panel"
              disabled={visualSaving}
              onClick={() => void select(undefined)}
            >
              <PanelRightCloseIcon aria-hidden="true" />
            </Button>
          </header>
          <div className="min-h-0 flex-1 overflow-y-auto p-4">
            {!loaded && !error && (
              <p role="status" className="text-sm text-muted-foreground">
                Loading MCP…
              </p>
            )}
            {loaded && (
              <>
                <p className="mb-4 text-sm text-muted-foreground">
                  Keep secrets out of the file: use{" "}
                  <code className="font-mono">{"{env:VAR_NAME}"}</code>.
                </p>
                <McpFields draft={draft} onChange={setDraft} />
                {typeof selected === "string" ? (
                  <div className="mt-4">
                    <UsedByRow refs={usedBy.of(selected)} project={usedBy.project} />
                  </div>
                ) : null}
              </>
            )}
            {error && (
              <div
                role="alert"
                className="mt-4 flex flex-col items-start gap-3 text-sm"
              >
                <p className="[overflow-wrap:anywhere] text-destructive">
                  {error}
                </p>
                {!loaded && typeof selected === "string" && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      setError(undefined)
                      load(selected)
                    }}
                  >
                    Retry
                  </Button>
                )}
              </div>
            )}
          </div>
          {typeof selected === "string" && loaded && (
            <footer className="shrink-0 border-t p-3">
              {actions(selected, "footer")}
            </footer>
          )}
        </aside>
      )}
      {discard.dialog}
    </div>
  )
}

export function McpsError({
  error,
  header: Header,
}: {
  error: unknown
  header: (props: { action?: ReactNode }) => ReactNode
}) {
  const router = useRouter()
  return (
    <div className="flex flex-col gap-6">
      <Header />
      <div className="flex flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
        <p className="text-sm [overflow-wrap:anywhere]">
          Couldn’t load MCPs: {String(error)}
        </p>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void router.invalidate()}
        >
          Retry
        </Button>
      </div>
    </div>
  )
}
