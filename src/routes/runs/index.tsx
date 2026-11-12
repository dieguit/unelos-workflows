import { useRef, useState } from "react"
import { createFileRoute, useNavigate, useRouter } from "@tanstack/react-router"
import { HistoryIcon, SearchXIcon } from "lucide-react"
import { z } from "zod"
import { EmptyState } from "@/components/empty-state"
import { ListActions } from "@/components/list-actions"
import { ListSearch } from "@/components/list-search"
import { PageHeader } from "@/components/page-header"
import { Button } from "@/components/ui/button"
import { requireProject } from "@/lib/projects"
import { RunsTable } from "./-components/runs-table"
import { usePolled } from "./-hooks/use-polled"
import { filterRuns } from "./-lib/format"
import { deleteRun, listRuns, type RunListItem } from "./-lib/runs-api"

export const Route = createFileRoute("/runs/")({
  beforeLoad: requireProject,
  component: RunsHome,
})

function Header() {
  return (
    <PageHeader
      title="Runs"
      description="Workflow runs in this project. Open one to follow its progress and read its artifacts."
    />
  )
}

function Heading({ children, count }: { children: string; count: number }) {
  return (
    <h2 className="flex items-baseline gap-2 text-lg font-semibold">
      {children}
      <span className="text-sm font-normal text-muted-foreground tabular-nums">{count}</span>
    </h2>
  )
}

function RunsHome() {
  const { project } = Route.useRouteContext()
  const navigate = useNavigate()
  const router = useRouter()
  const { data, error, refresh } = usePolled(() => listRuns(project.path), project.path)
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)

  if (!data)
    return error ? (
      <div className="flex flex-col gap-6">
        <Header />
        <div className="flex flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
          <p className="text-sm">Couldn’t load runs: {error}</p>
          <Button variant="outline" size="sm" onClick={() => void refresh()}>
            Retry
          </Button>
        </div>
      </div>
    ) : (
      <div role="status" className="flex flex-col gap-6">
        <Header />
        <p>Loading runs…</p>
      </div>
    )

  if (data.length === 0)
    return (
      <div className="flex flex-col gap-6">
        <Header />
        <EmptyState
          dashed
          icon={HistoryIcon}
          title="No runs yet"
          description="Runs appear here once a workflow is started in this project."
        />
      </div>
    )

  const shown = filterRuns(data, query)
  const active = shown.filter((r) => r.status !== "finished")
  const finished = shown.filter((r) => r.status === "finished")
  const table = (runs: RunListItem[], isFinished?: boolean) => (
    <RunsTable
      runs={runs}
      finished={isFinished}
      onOpen={(runId) => void navigate({ to: "/runs/$runId", params: { runId } })}
      actions={(run) => (
        <ListActions
          kind="Run"
          name={run.name}
          nameSchema={z.string()}
          remove={async () => {
            await deleteRun(project.path, run.runId)
            await refresh()
            void router.invalidate()
          }}
        />
      )}
    />
  )

  return (
    <div className="flex flex-col gap-4">
      <Header />
      <ListSearch
        noun="run"
        value={query}
        onChange={setQuery}
        shown={shown.length}
        total={data.length}
        inputRef={search}
      />
      {shown.length === 0 ? (
        <EmptyState
          icon={SearchXIcon}
          title={`No runs match “${query.trim()}”`}
          action={
            <Button variant="outline" size="sm" onClick={() => setQuery("")}>
              Clear search
            </Button>
          }
        />
      ) : (
        <>
          <section aria-label="Active runs" className="flex flex-col gap-2">
            <Heading count={active.length}>Active</Heading>
            {active.length ? table(active) : <p className="text-sm text-muted-foreground">No active runs.</p>}
          </section>
          <section aria-label="Finished runs" className="mt-4 flex flex-col gap-2">
            <Heading count={finished.length}>Finished</Heading>
            {finished.length ? table(finished, true) : <p className="text-sm text-muted-foreground">No finished runs.</p>}
          </section>
        </>
      )}
    </div>
  )
}
