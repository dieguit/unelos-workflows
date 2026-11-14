import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import { useEffect, useRef, useState, type ReactNode } from "react"
import { createFileRoute, useBlocker, useRouter } from "@tanstack/react-router"
import { useStore } from "@tanstack/react-form"
import { PlusIcon } from "lucide-react"
import { toast } from "sonner"
import { z } from "zod"
import { Button } from "@/components/ui/button"
import { ListSearch } from "@/components/list-search"
import { PageHeader } from "@/components/page-header"
import {
  deleteModelMapping,
  listModelMappings,
  saveModelMapping,
  usedBy,
} from "@/lib/model-mappings"
import { autoBuildHarness, listAgents } from "@/lib/projects"
import { listHarnesses, sortHarnesses } from "@/lib/harnesses"
import { MappingInspector } from "./-components/mapping-inspector"
import { MappingsTable } from "./-components/mappings-table"
import { useHarnessModels } from "@/hooks/use-harness-models"
import { filterMappings } from "./-lib/model-mappings-list"
import { useMappingForm } from "./-hooks/use-mapping-form"
import { useDiscardChanges } from "@/hooks/use-discard-changes"
import { useProjectDiscardGuard } from "@/hooks/use-project-discard-guard"

export const Route = createFileRoute("/model-mappings/")({
  validateSearch: z.object({ alias: z.string().optional().catch(undefined) }),
  loader: async ({ context }) => ({
    mappings: await listModelMappings(),
    harnesses: sortHarnesses((await listHarnesses()).filter((h) => h.installed)),
    // "Used by" is per open project; a failed agent scan shouldn't hide the page.
    agents: context.project
      ? await listAgents(context.project.path).catch(() => null)
      : null,
  }),
  component: ModelMappingsHome,
  pendingComponent: () => (
    <div className="flex flex-col gap-6">
      <Header />
      <p role="status">Loading model mappings…</p>
    </div>
  ),
  errorComponent: ModelMappingsError,
})

function Header({ action }: { action?: ReactNode }) {
  return (
    <PageHeader
      title="Model mappings"
      description="Global aliases agents use instead of concrete models. Map each alias for every configured harness."
      action={action}
    />
  )
}

function ModelMappingsHome() {
  const loaded = Route.useLoaderData()
  const [mappings, setMappings] = useState(loaded.mappings)
  const { agents, harnesses } = loaded
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)
  const { alias } = Route.useSearch()
  const navigate = Route.useNavigate()
  const [creating, setCreating] = useState(false)
  const harnessModels = useHarnessModels()
  const selected = alias && mappings[alias] ? alias : undefined
  const total = Object.keys(mappings).length
  const [error, setError] = useState<string>()
  const [deleting, setDeleting] = useState(false)
  const discard = useDiscardChanges()
  const { project } = Route.useRouteContext()
  // Mappings are global; regenerate the open project's harness files so a fixed mapping takes effect.
  const rebuild = () =>
    project &&
    autoBuildHarness(project.path).catch((e) =>
      toast.error(`Saved, but harness config build failed: ${String(e)}`)
    )
  const form = useMappingForm(
    {
      alias: creating ? "" : (selected ?? ""),
      models: creating || !selected ? {} : mappings[selected],
    },
    async (draft) => {
      setError(undefined)
      const n = usedBy(agents, selected ?? "")?.length ?? 0
      if (
        selected &&
        draft.alias !== selected &&
        n &&
        !window.confirm(
          `${n} agent${n === 1 ? "" : "s"} use “${selected}” and will keep pointing at the old name. Rename anyway?`
        )
      )
        return
      try {
        const saved = await saveModelMapping(
          creating ? null : selected!,
          draft.alias,
          draft.models
        )
        setMappings(saved)
        void rebuild()
        if (creating || draft.alias !== selected) {
          setCreating(false)
          void navigate({ search: { alias: draft.alias }, ignoreBlocker: true })
        }
        toast.success(
          creating ? `Created ${draft.alias}` : `Saved ${draft.alias}`
        )
        return { alias: draft.alias, models: saved[draft.alias] }
      } catch (error) {
        setError(String(error))
      }
    }
  )
  const isDefaultValue = useStore(form.store, (s) => s.isDefaultValue)
  const submitting = useStore(form.store, (s) => s.isSubmitting)
  const busy = submitting || deleting
  const visualBusy = useDelayedFlag(busy)
  const dirty = !isDefaultValue
  const [errorContext, setErrorContext] = useState({ selected, creating, mappings })
  if (errorContext.selected !== selected || errorContext.creating !== creating || errorContext.mappings !== mappings) {
    setErrorContext({ selected, creating, mappings })
    setError(undefined)
  }
  useEffect(() => {
    form.reset({
      alias: creating ? "" : (selected ?? ""),
      models: creating || !selected ? {} : mappings[selected],
    })
  }, [selected, creating, mappings, form])
  const projectChangeApproved = useProjectDiscardGuard(busy, dirty, discard.ask)
  useBlocker({
    shouldBlockFn: async () => !projectChangeApproved.current && (busy || (dirty && !(await discard.ask()))),
    enableBeforeUnload: dirty || busy,
  })

  const select = async (next?: string) => {
    if (busy || (!creating && selected === next)) return
    if (dirty && !(await discard.ask())) return
    setCreating(false)
    void navigate({ search: next ? { alias: next } : {}, ignoreBlocker: true })
  }
  const create = async () => {
    if (busy || creating) return
    if (dirty && !(await discard.ask())) return
    setCreating(true)
    void navigate({ search: {}, ignoreBlocker: true })
  }

  return (
    // Cancel <main>'s padding so the inspector sits flush right at full height.
    <div inert={busy} className="-mx-8 -mt-6 -mb-8 flex h-[calc(100%+3.5rem)]">
      <div className="min-w-0 flex-1 overflow-y-auto px-8 pt-6 pb-8">
        <div className="flex flex-col gap-4">
          <Header
            action={
              <Button onClick={() => void create()} disabled={visualBusy}>
                <PlusIcon aria-hidden="true" />
                New alias
              </Button>
            }
          />
          {total > 0 ? (
            <ListSearch
              noun="alias"
              value={query}
              onChange={setQuery}
              shown={filterMappings(mappings, query, harnesses).length}
              total={total}
              inputRef={search}
            />
          ) : null}
          <MappingsTable
            harnesses={harnesses}
            mappings={mappings}
            agents={agents}
            query={query}
            onClear={() => {
              setQuery("")
              search.current?.focus()
            }}
            onCreate={create}
            selected={creating ? undefined : selected}
            onSelect={select}
          />
        </div>
      </div>
      {creating || selected ? (
        <MappingInspector
          harnesses={harnesses}
          key={creating ? "\0new" : selected}
          original={creating ? null : selected!}
          form={form}
          dirty={dirty}
          busy={busy}
          error={error}
          usedBy={creating ? null : usedBy(agents, selected!)}
          harnessModels={harnessModels.models}
          onLoadModels={harnessModels.load}
          onDelete={async () => {
            if (busy) return
            setDeleting(true)
            setError(undefined)
            try {
              setMappings(await deleteModelMapping(selected!))
              void rebuild()
              toast.success(`Deleted ${selected}`)
              setCreating(false)
              void navigate({ search: {}, ignoreBlocker: true })
            } catch (error) {
              setError(String(error))
            } finally {
              setDeleting(false)
            }
          }}
          onClose={() => select()}
        />
      ) : null}
      {discard.dialog}
    </div>
  )
}

function ModelMappingsError({ error }: { error: unknown }) {
  const router = useRouter()
  return (
    <div className="flex flex-col gap-6">
      <Header />
      <div className="flex flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
        <p className="text-sm [overflow-wrap:anywhere]">
          Couldn’t load model mappings:{" "}
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
