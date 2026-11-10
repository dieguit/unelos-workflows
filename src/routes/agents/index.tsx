import { useRef, useState, type ReactNode } from "react"
import { createFileRoute, useBlocker, useRouter } from "@tanstack/react-router"
import { PlusIcon } from "lucide-react"
import { useStore } from "@tanstack/react-form"
import { BodyEditor } from "@/components/body-editor"
import { Button } from "@/components/ui/button"
import { useDiscardChanges } from "@/hooks/use-discard-changes"
import { ListSearch } from "@/components/list-search"
import { PageHeader } from "@/components/page-header"
import {
  getAgent,
  listAgents,
  requireProject,
  saveAgent,
  type AgentDocument,
} from "@/lib/projects"
import { AgentInspector } from "./-components/agent-inspector"
import { AgentsList, filterAgents } from "./-components/agents-list"
import { emptyAgent, useAgentForm } from "./-hooks/use-agent-form"

export const Route = createFileRoute("/agents/")({
  beforeLoad: requireProject,
  loader: async ({ context }) => ({
    root: context.project.path,
    agents: await listAgents(context.project.path),
  }),
  component: AgentsRoute,
  pendingComponent: () => <p role="status">Loading agents…</p>,
  errorComponent: AgentsError,
})

function Header({ action }: { action?: ReactNode }) {
  return (
    <PageHeader
      title="Agents"
      description="Agent definitions for this project."
      action={action}
    />
  )
}

function AgentsRoute() {
  const { project } = Route.useRouteContext()
  return <AgentsHome key={project.path} />
}

function AgentsHome() {
  const { root, agents } = Route.useLoaderData()
  const { project } = Route.useRouteContext()
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)
  const router = useRouter()
  // undefined = closed, null = new draft, string = existing identifier.
  const [selected, setSelected] = useState<string | null>()
  const [loaded, setLoaded] = useState(false)
  const [baseline, setBaseline] = useState<AgentDocument>(emptyAgent)
  const [loadError, setLoadError] = useState<string>()
  const [saveError, setSaveError] = useState<string>()
  const [editor, setEditor] = useState<string | null>(null)
  const discard = useDiscardChanges()
  const form = useAgentForm(
    baseline,
    async (updated, closeEditor) => {
      setSaveError(undefined)
      try {
        const saved = await saveAgent(root, selected ?? null, updated)
        setBaseline(saved)
        setSelected(saved.id)
        if (closeEditor) setEditor(null)
        void router.invalidate()
        return saved
      } catch (error) {
        setSaveError(String(error))
      }
    },
    () =>
      setSaveError(
        "Check the highlighted fields. Enter a Name (id) before saving; Cancel instructions editing to return. Your text is kept."
      )
  )
  const draft = useStore(form.store, (s) => s.values)
  const saving = useStore(form.store, (s) => s.isSubmitting)
  const isDefaultValue = useStore(form.store, (s) => s.isDefaultValue)
  const request = useRef(0)
  const dirty = loaded && !isDefaultValue
  const editorDirty = editor !== null && editor !== draft?.instructions
  const askDiscard = discard.ask
  useBlocker({
    shouldBlockFn: async () =>
      saving || ((dirty || editorDirty) && !(await askDiscard())),
    enableBeforeUnload: dirty || editorDirty || saving,
  })

  const load = async (id: string) => {
    const token = ++request.current
    setLoaded(false)
    setLoadError(undefined)
    try {
      const doc = await getAgent(root, id)
      if (token !== request.current) return
      form.reset(doc)
      setBaseline(doc)
      setLoaded(true)
    } catch (error) {
      if (token === request.current) setLoadError(String(error))
    }
  }
  const select = async (id: string | null | undefined) => {
    if (saving || id === selected) return
    if ((dirty || editorDirty) && !(await askDiscard())) return
    ++request.current
    setSelected(id)
    setEditor(null)
    setSaveError(undefined)
    setLoadError(undefined)
    setLoaded(false)
    form.reset(emptyAgent)
    setBaseline(emptyAgent)
    if (typeof id === "string") void load(id)
    else if (id === null) setLoaded(true)
  }

  if (root !== project.path)
    return (
      <div className="flex flex-col gap-6">
        <Header />
        <p role="status">Loading agents…</p>
      </div>
    )

  return (
    <div className="-mx-8 -mt-6 -mb-8 flex h-[calc(100%+3.5rem)]">
      <div className="min-w-0 flex-1 overflow-y-auto px-8 pt-6 pb-8">
        {editor !== null && draft ? (
          <div className="flex h-full flex-col gap-4">
            {saveError ? (
              <div role="alert" className="flex flex-col items-start gap-2">
                <p className="text-sm text-destructive">
                  Could not save Agent. Check the error below and try Save and
                  close again.
                </p>
                <p className="text-xs [overflow-wrap:anywhere] text-muted-foreground">
                  {saveError}
                </p>
              </div>
            ) : null}
            <BodyEditor
              title={draft.id || "New Agent"}
              label="Instructions"
              body={editor}
              onChange={setEditor}
              onCancel={() => {
                void (async () => {
                  if (
                    !editorDirty ||
                    (await askDiscard(
                      "Discard unsaved changes to instructions? The Agent draft will be kept."
                    ))
                  )
                    setEditor(null)
                })()
              }}
              onSave={() =>
                form.handleSubmit({ instructions: editor, closeEditor: true })
              }
              onBlur={() => {
                void form.validateField("instructions", "blur")
              }}
            />
          </div>
        ) : (
          <div className="flex flex-col gap-4">
            <Header
              action={
                <Button onClick={() => void select(null)} disabled={saving}>
                  <PlusIcon aria-hidden="true" />
                  New Agent
                </Button>
              }
            />
            {agents.length > 0 ? (
              <ListSearch
                noun="agent"
                value={query}
                onChange={setQuery}
                shown={filterAgents(agents, query).length}
                total={agents.length}
                inputRef={search}
              />
            ) : null}
            <AgentsList
              agents={agents}
              selected={selected ?? undefined}
              onSelect={(id) => void select(id)}
              onCreate={() => void select(null)}
              query={query}
              onClear={() => {
                setQuery("")
                search.current?.focus()
              }}
            />
          </div>
        )}
      </div>
      {selected !== undefined ? (
        <AgentInspector
          key={selected ?? "\0new"}
          root={root}
          originalId={selected}
          form={form}
          draft={loaded ? draft : undefined}
          dirty={dirty}
          loading={!loaded && !loadError}
          loadError={loadError}
          saveError={editor !== null ? undefined : saveError}
          saving={saving}
          editing={editor !== null}
          onEdit={() => draft && setEditor(draft.instructions)}
          onClose={() => void select(undefined)}
          onRetry={() => typeof selected === "string" && void load(selected)}
        />
      ) : null}
      {discard.dialog}
    </div>
  )
}

function AgentsError({ error }: { error: unknown }) {
  const router = useRouter()
  return (
    <div className="flex flex-col gap-6">
      <Header />
      <div className="flex flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
        <p className="text-sm [overflow-wrap:anywhere]">
          Couldn’t load agents:{" "}
          {error instanceof Error ? error.message : String(error)}
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
