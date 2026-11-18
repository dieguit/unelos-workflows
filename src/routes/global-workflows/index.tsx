import { useRef, useState } from "react"
import { toast } from "sonner"
import { createFileRoute, useRouter } from "@tanstack/react-router"
import { Button } from "@/components/ui/button"
import { ListActions } from "@/components/list-actions"
import { ListSearch } from "@/components/list-search"
import { PageHeader } from "@/components/page-header"
import { NoResults, NoWorkflows } from "@/components/workflows-empty"
import { NewWorkflowButton, WorkflowsTable } from "@/components/workflows-table"
import { WorkflowEditor } from "@/components/workflow-editor/workflow-editor"
import { stripGlobal } from "@/lib/global-ref"
import { filterWorkflows } from "@/lib/workflow-filter"
import { workflowSettingsSchema } from "@/lib/workflow-settings-schema"
import {
  createGlobalWorkflow,
  duplicateGlobalWorkflow,
  listGlobalWorkflows,
  removeGlobalWorkflow,
} from "@/lib/projects"

export const Route = createFileRoute("/global-workflows/")({
  loader: async () => ({ workflows: await listGlobalWorkflows() }),
  component: GlobalWorkflowsPage,
  pendingComponent: () => <p role="status">Loading global workflows…</p>,
  errorComponent: GlobalWorkflowsError,
})

function Header({ onCreate, creating }: { onCreate?: () => void; creating?: boolean }) {
  return (
    <PageHeader
      title="Global workflows"
      description="Workflows available to every project on this machine. They can only use global agents, tasks, skills and workflows. Stored in ~/.config/unelos."
      action={<NewWorkflowButton onClick={onCreate} disabled={creating} />}
    />
  )
}

function GlobalWorkflowsPage() {
  const { workflows: loaded } = Route.useLoaderData()
  const { project } = Route.useRouteContext()
  const root = project?.path ?? null
  // Names and files shown without the global/ prefix: every row on this page is global.
  const workflows = loaded.map((w) => ({ ...w, name: stripGlobal(w.name), file: stripGlobal(w.file) }))
  const [query, setQuery] = useState("")
  const [selected, setSelected] = useState<string>()
  const [creating, setCreating] = useState(false)
  const search = useRef<HTMLInputElement>(null)
  const router = useRouter()

  async function create() {
    if (creating) return
    setCreating(true)
    try {
      setSelected(await createGlobalWorkflow())
      void router.invalidate()
    } catch (error) {
      toast.error(`Could not create workflow: ${String(error)}`)
    } finally {
      setCreating(false)
    }
  }

  if (selected !== undefined)
    return <WorkflowEditor key={selected} root={root} id={selected} global onBack={() => setSelected(undefined)} />

  const shown = filterWorkflows(workflows, query)
  const idOf = (file: string) => file.replace(/\.ya?ml$/i, "")
  return (
    <div className="flex flex-col gap-4">
      <Header onCreate={create} creating={creating} />
      {workflows.length === 0 ? (
        <NoWorkflows
          onCreate={create}
          creating={creating}
          description="Global workflows are shared by every project on this machine. They can only use global agents, tasks, skills and workflows."
        />
      ) : (
        <>
          <ListSearch
            noun="workflow"
            value={query}
            onChange={setQuery}
            shown={shown.length}
            total={workflows.length}
            inputRef={search}
          />
          {shown.length === 0 ? (
            <NoResults query={query.trim()} onClear={() => setQuery("")} />
          ) : (
            <WorkflowsTable
              workflows={shown}
              onOpen={(file) => setSelected(idOf(file))}
              actions={(workflow) => {
                const id = idOf(workflow.file)
                return (
                  <ListActions
                    kind="Workflow"
                    name={workflow.name}
                    nameSchema={workflowSettingsSchema("").shape.name}
                    duplicate={async (newId) => {
                      await duplicateGlobalWorkflow(id, newId)
                      void router.invalidate()
                    }}
                    remove={async () => {
                      await removeGlobalWorkflow(root, id)
                      void router.invalidate()
                    }}
                  />
                )
              }}
            />
          )}
        </>
      )}
    </div>
  )
}

function GlobalWorkflowsError({ error }: { error: unknown }) {
  const router = useRouter()
  return (
    <div className="flex flex-col gap-6">
      <Header />
      <div className="flex flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
        <p className="text-sm [overflow-wrap:anywhere]">
          Couldn’t load global workflows: {error instanceof Error ? error.message : String(error)}
        </p>
        <Button variant="outline" size="sm" onClick={() => void router.invalidate()}>
          Retry
        </Button>
      </div>
    </div>
  )
}
