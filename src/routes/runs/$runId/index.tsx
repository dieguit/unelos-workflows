import { createFileRoute, Link } from "@tanstack/react-router"
import { Button } from "@/components/ui/button"
import { requireProject } from "@/lib/projects"
import { usePolled } from "../-hooks/use-polled"
import { getRun } from "../-lib/runs-api"
import { RunView } from "./-components/run-view"

export const Route = createFileRoute("/runs/$runId/")({
  beforeLoad: requireProject,
  component: RunPage,
})

function RunPage() {
  const { runId } = Route.useParams()
  const { project } = Route.useRouteContext()
  const { data, error, refresh } = usePolled(() => getRun(project.path, runId), `${project.path}:${runId}`)
  if (data) return <RunView root={project.path} run={data} />
  return (
    <div className="flex flex-col items-start gap-4">
      <Button variant="ghost" nativeButton={false} render={<Link to="/runs" />}>
        Runs
      </Button>
      {error ? (
        <div role="alert" className="space-y-3">
          <p>Could not load run: {error}</p>
          <Button variant="outline" onClick={() => void refresh()}>
            Retry
          </Button>
        </div>
      ) : (
        <p role="status">Loading run…</p>
      )}
    </div>
  )
}
