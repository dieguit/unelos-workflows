import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import { useRef, useState, type ReactNode } from "react"
import { createFileRoute, useBlocker, useRouter } from "@tanstack/react-router"
import { PlusIcon } from "lucide-react"
import { useStore } from "@tanstack/react-form"
import { BodyEditor } from "@/components/body-editor"
import { Button } from "@/components/ui/button"
import { useDiscardChanges } from "@/hooks/use-discard-changes"
import { useProjectDiscardGuard } from "@/hooks/use-project-discard-guard"
import { applyBodyDraft } from "@/lib/apply-body-draft"
import { ListSearch } from "@/components/list-search"
import { PageHeader } from "@/components/page-header"
import { ListActions } from "@/components/list-actions"
import {
  getAgent,
  duplicateAgent,
  listAgents,
  requireProject,
  saveAgent,
  removeAgent,
  type AgentDocument,
} from "@/lib/projects"
import { loadReferences, usedByFor } from "@/lib/references"
import { AgentInspector } from "./-components/agent-inspector"
import { AgentsList, filterAgents } from "./-components/agents-list"
import { emptyAgent, useAgentForm } from "./-hooks/use-agent-form"
import { agentSchema } from "./-lib/agent-schema"

export const Route = createFileRoute("/agents/")({
  beforeLoad: requireProject,
  loader: async ({ context }) => ({
    root: context.project.path,
    agents: await listAgents(context.project.path),
    references: await loadReferences(context.project.path),
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
  const { root, agents, references } = Route.useLoaderData()
  const usedBy = usedByFor(references?.agents)
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
  const visualSaving = useDelayedFlag(saving)
  const isDefaultValue = useStore(form.store, (s) => s.isDefaultValue)
  const request = useRef(0)
  const dirty = loaded && !isDefaultValue
  const editorDirty = editor !== null && editor !== draft?.instructions
  const askDiscard = discard.ask
  const projectChangeApproved = useProjectDiscardGuard(saving, dirty || editorDirty, askDiscard)
  useBlocker({
    shouldBlockFn: async () =>
      !projectChangeApproved.current && (saving || ((dirty || editorDirty) && !(await askDiscard()))),
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

  const closeIf = (id: string) => {
    if (selected !== id) return
    ++request.current
    setSelected(undefined)
    setEditor(null)
    setLoaded(false)
    form.reset(emptyAgent)
    setBaseline(emptyAgent)
  }
  const actions = (id: string, variant?: "footer") => (
    <ListActions
      variant={variant}
      kind="Agent"
      name={id}
      nameSchema={agentSchema.shape.id}
      disabled={visualSaving}
      beforeRemove={async () =>
        selected !== id ||
        (!saving && (!(dirty || editorDirty) || (await askDiscard())))
      }
      duplicate={async (newId) => {
        await duplicateAgent(root, id, newId)
        void router.invalidate()
      }}
      remove={async () => {
        await removeAgent(root, id)
        closeIf(id)
        void router.invalidate()
      }}
    />
  )

  if (root !== project.path)
    return (
      <div className="flex flex-col gap-6">
        <Header />
        <p role="status">Loading agents…</p>
      </div>
    )

  return (
    <div inert={saving} className="-mx-8 -mt-6 -mb-8 flex h-[calc(100%+3.5rem)]">
      <div className="min-w-0 flex-1 overflow-y-auto px-8 pt-6 pb-8">
        {editor !== null && draft ? (
          <div className="flex h-full flex-col gap-4">
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
              onSave={async () => {
                applyBodyDraft(editor, (body) => form.setFieldValue("instructions", body), () => setEditor(null))
              }}
              onBlur={() => {
                void form.validateField("instructions", "blur")
              }}
            />
          </div>
        ) : (
          <div className="flex flex-col gap-4">
            <Header
              action={
                <Button onClick={() => void select(null)} disabled={visualSaving}>
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
              usedBy={usedBy}
              selected={selected ?? undefined}
              onSelect={(id) => void select(id)}
              onCreate={() => void select(null)}
              actions={(id) => actions(id)}
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
          usedBy={typeof selected === "string" ? usedBy.of(selected) : null}
          originalId={selected}
          form={form}
          footer={typeof selected === "string" && loaded ? actions(selected, "footer") : undefined}
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
