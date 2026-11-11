import { useRef, useState } from "react"
import { createFileRoute, useRouter } from "@tanstack/react-router"
import { Button } from "@/components/ui/button"
import { ListSearch } from "@/components/list-search"
import { PageHeader } from "@/components/page-header"
import { ListActions } from "@/components/list-actions"
import {
  duplicateWorkflow,
  listWorkflows,
  removeWorkflow,
  requireProject,
} from "@/lib/projects"
import { filterWorkflows } from "./-lib/filter"
import { workflowSettingsSchema } from "./-lib/workflow-settings-schema"
import {
  NewWorkflowButton,
  WorkflowsTable,
} from "./-components/workflows-table"
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

function Header() {
  return (
    <PageHeader
      title="Workflows"
      description="Reusable process definitions for this project. Open one to edit its settings."
      action={<NewWorkflowButton />}
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
  const shown = filterWorkflows(workflows, query)

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
        <Header />
        <NoWorkflows />
      </div>
    )

  return (
    <div className="flex flex-col gap-4">
      <Header />
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
