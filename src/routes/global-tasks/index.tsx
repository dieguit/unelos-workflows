import { useCallback, type ReactNode } from "react"
import { createFileRoute } from "@tanstack/react-router"
import { z } from "zod"
import { PageHeader } from "@/components/page-header"
import { TasksError, TasksPage } from "@/components/tasks-page"
import {
  duplicateGlobalTask,
  getGlobalTask,
  listGlobalTasks,
  listSkills,
  removeGlobalTask,
  saveGlobalTask,
} from "@/lib/projects"

export const Route = createFileRoute("/global-tasks/")({
  // Opens a task directly, e.g. from a workflow step that uses it.
  validateSearch: z.object({ task: z.string().optional().catch(undefined) }),
  loader: async () => ({ tasks: await listGlobalTasks() }),
  component: GlobalTasksRoute,
  pendingComponent: () => <p role="status">Loading global tasks…</p>,
  errorComponent: ({ error }) => <TasksError error={error} header={Header} />,
})

function Header({ action }: { action?: ReactNode }) {
  return (
    <PageHeader
      title="Global tasks"
      description="Tasks available to every project on this machine. Stored in ~/.config/unelos."
      action={action}
    />
  )
}

const api = {
  get: getGlobalTask,
  save: saveGlobalTask,
  duplicate: duplicateGlobalTask,
  remove: removeGlobalTask,
}

function GlobalTasksRoute() {
  const { tasks } = Route.useLoaderData()
  const { task } = Route.useSearch()
  const { project } = Route.useRouteContext()
  const root = project?.path
  // Skill names resolve per project; the open project (if any) supplies the picker list.
  const fetchSkills = useCallback(() => listSkills(root!), [root])
  return (
    <TasksPage
      tasks={tasks}
      initial={task}
      fetchSkills={root ? fetchSkills : null}
      scope={{ path: "global", label: "Global Task" }}
      header={Header}
      api={api}
    />
  )
}
