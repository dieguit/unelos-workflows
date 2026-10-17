import { useRef, useState } from "react"
import { createFileRoute, useRouter } from "@tanstack/react-router"
import { Button } from "@/components/ui/button"
import { ListSearch } from "@/components/list-search"
import { PageHeader } from "@/components/page-header"
import { listWorkflows, requireProject } from "@/lib/projects"
import { filterWorkflows } from "./-lib/filter"
import {
  NewWorkflowButton,
  WorkflowsTable,
} from "./-components/workflows-table"
import { NoResults, NoWorkflows } from "./-components/workflows-empty"

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
      description="Reusable process definitions for this project. Open one to edit its steps, routes and inputs."
      action={<NewWorkflowButton />}
    />
  )
}

function WorkflowsHome() {
  const data = Route.useLoaderData()
  const { project } = Route.useRouteContext()
  const workflows = data.root === project.path ? data.workflows : []
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)
  const shown = filterWorkflows(workflows, query)

  if (data.root !== project.path)
    return (
      <div role="status" className="flex flex-col gap-6">
        <Header />
        <p>Loading workflows…</p>
      </div>
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
        <WorkflowsTable workflows={shown} />
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
