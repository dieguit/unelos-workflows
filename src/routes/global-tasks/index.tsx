import { listGlobalPrefixedSkills } from "@/hooks/skill-fetchers"
import { type ReactNode } from "react"
import { createFileRoute } from "@tanstack/react-router"
import { z } from "zod"
import { PageHeader } from "@/components/page-header"
import { loadReferences, usedByFor } from "@/lib/references"
import { TasksError, TasksPage } from "@/components/tasks-page"
import {
  duplicateGlobalTask,
  getGlobalTask,
  listGlobalTasks,
  removeGlobalTask,
  saveGlobalTask,
} from "@/lib/projects"

export const Route = createFileRoute("/global-tasks/")({
  // Opens a task directly, e.g. from a workflow step that uses it.
  validateSearch: z.object({ task: z.string().optional().catch(undefined) }),
  loader: async ({ context }) => ({
    tasks: await listGlobalTasks(),
    references: await loadReferences(context.project?.path),
  }),
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
  const { tasks, references } = Route.useLoaderData()
  const { task } = Route.useSearch()
  const { project } = Route.useRouteContext()
  return (
    <TasksPage
      tasks={tasks}
      usedBy={usedByFor(references?.tasks, { global: true, project: project?.name })}
      initial={task}
      fetchSkills={listGlobalPrefixedSkills}
      scope={{ path: "global", label: "Global Task" }}
      header={Header}
      api={api}
    />
  )
}
