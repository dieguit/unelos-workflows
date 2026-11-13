import { useEffect, useRef, useState } from "react"
import { TriangleAlertIcon } from "lucide-react"
import { toast } from "sonner"
import { createFileRoute, useRouter } from "@tanstack/react-router"
import { Button } from "@/components/ui/button"
import { ListSearch } from "@/components/list-search"
import { PageHeader } from "@/components/page-header"
import { ListActions } from "@/components/list-actions"
import {
  duplicateWorkflow,
  createWorkflow,
  listWorkflows,
  removeWorkflow,
  requireProject,
} from "@/lib/projects"
import { filterWorkflows } from "./-lib/filter"
import { workflowSettingsSchema } from "./-lib/workflow-settings-schema"
import {
  ImportWorkflowButton,
  NewWorkflowButton,
  WorkflowsTable,
} from "./-components/workflows-table"
import { ImportDialog } from "./-components/import-dialog"
import { NoResults, NoWorkflows } from "./-components/workflows-empty"
import { WorkflowEditor } from "./-components/workflow-editor"

export const Route = createFileRoute("/workflows/")({
  beforeLoad: requireProject,
  loader: async ({ context }) => ({
    root: context.project.path,
    workflows: await listWorkflows(context.project.path),
  }),
  component: WorkflowsHome,
  errorComponent: WorkflowsError,
})

function Header({ onCreate, creating, onImport }: { onCreate?: () => void; creating?: boolean; onImport?: () => void }) {
  return (
    <PageHeader
      title="Workflows"
      description="Reusable process definitions for this project. Open one to edit its settings."
      action={
        <div className="flex gap-2">
          <ImportWorkflowButton onClick={onImport} />
          <NewWorkflowButton onClick={onCreate} disabled={creating} />
        </div>
      }
    />
  )
}

function WorkflowsHome() {
  const data = Route.useLoaderData()
  const { project } = Route.useRouteContext()
  const workflows = data.root === project.path ? data.workflows : []
  const [query, setQuery] = useState("")
  const [selected, setSelected] = useState<{ root: string; id: string }>()
  const search = useRef<HTMLInputElement>(null)
  const router = useRouter()
  const [creating, setCreating] = useState(false)
  async function create() {
    if (creating) return
    setCreating(true)
    try {
      const id = await createWorkflow(data.root)
      setSelected({ root: data.root, id })
      void router.invalidate()
    } catch (error) {
      toast.error(`Could not create workflow: ${String(error)}`)
    } finally {
      setCreating(false)
    }
  }
  const [importing, setImporting] = useState(false)
  const [highlight, setHighlight] = useState<string>()
  useEffect(() => {
    if (!highlight) return
    const timer = setTimeout(() => setHighlight(undefined), 2000)
    return () => clearTimeout(timer)
  }, [highlight])
  const shown = filterWorkflows(workflows, query)
  const importDialog = importing ? (
    <ImportDialog
      root={data.root}
      onClose={() => setImporting(false)}
      onImported={async ({ workflow, warningCount }) => {
        setImporting(false)
        setQuery("")
        await router.invalidate()
        setHighlight(`${workflow}.yaml`)
        toast.success(
          <>
            Imported <code className="font-mono">{workflow}</code>
          </>,
          {
            description:
              warningCount > 0 ? (
                <span className="flex items-center gap-1.5 text-warning">
                  <TriangleAlertIcon aria-hidden className="size-3.5" />
                  {warningCount} {warningCount === 1 ? "warning" : "warnings"} to fix
                </span>
              ) : undefined,
            closeButton: true,
            duration: 8000,
            classNames: { actionButton: "!bg-primary-muted !text-primary" },
            action: { label: "Open", onClick: () => setSelected({ root: data.root, id: workflow }) },
          }
        )
      }}
    />
  ) : null

  if (data.root !== project.path)
    return (
      <div role="status" className="flex flex-col gap-6">
        <Header />
        <p>Loading workflows…</p>
      </div>
    )

  if (selected?.root === data.root)
    return (
      <WorkflowEditor
        key={`${selected.root}:${selected.id}`}
        root={data.root}
        id={selected.id}
        onBack={() => setSelected(undefined)}
      />
    )

  if (workflows.length === 0)
    return (
      <div className="flex flex-col gap-6">
        <Header onCreate={create} creating={creating} onImport={() => setImporting(true)} />
        <NoWorkflows onCreate={create} creating={creating} onImport={() => setImporting(true)} />
        {importDialog}
      </div>
    )

  return (
    <div className="flex flex-col gap-4">
      <Header onCreate={create} creating={creating} onImport={() => setImporting(true)} />
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
          highlight={highlight}
          onOpen={(file) =>
            setSelected({ root: data.root, id: file.replace(/\.ya?ml$/i, "") })
          }
          actions={(workflow) => {
            const id = workflow.file.replace(/\.ya?ml$/i, "")
            return (
              <ListActions
                kind="Workflow"
                name={workflow.name}
                nameSchema={workflowSettingsSchema("").shape.name}
                duplicate={async (newId) => {
                  await duplicateWorkflow(data.root, id, newId)
                  void router.invalidate()
                }}
                remove={async () => {
                  await removeWorkflow(data.root, id)
                  void router.invalidate()
                }}
              />
            )
          }}
        />
      )}
      {importDialog}
    </div>
  )
}

function WorkflowsError({ error }: { error: unknown }) {
  const router = useRouter()
  return (
    <div className="flex flex-col gap-6">
      <Header />
      <div className="flex flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
        <p className="text-sm">
          Couldn’t load workflows:{" "}
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
