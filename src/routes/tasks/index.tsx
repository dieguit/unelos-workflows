import { listAllSkills } from "@/hooks/skill-fetchers"
import { useCallback } from "react"
import { createFileRoute } from "@tanstack/react-router"
import { z } from "zod"
import { PageHeader } from "@/components/page-header"
import { TasksError, TasksPage } from "@/components/tasks-page"
import {
  duplicateTask,
  getTask,
  listTasks,
  makeTaskGlobal,
  removeTask,
  requireProject,
  saveTask,
} from "@/lib/projects"
import type { ReactNode } from "react"

export const Route = createFileRoute("/tasks/")({
  // Opens a task directly, e.g. from a workflow step that uses it.
  validateSearch: z.object({ task: z.string().optional().catch(undefined) }),
  beforeLoad: requireProject,
  loader: async ({ context }) => ({
    root: context.project.path,
    tasks: await listTasks(context.project.path),
  }),
  component: TasksRoute,
  pendingComponent: () => <p role="status">Loading tasks…</p>,
  errorComponent: ({ error }) => <TasksError error={error} header={Header} />,
})

function Header({ action }: { action?: ReactNode }) {
  return (
    <PageHeader
      title="Tasks"
      description="Reusable tasks for this project’s workflows."
      action={action}
    />
  )
}

function TasksRoute() {
  const { root, tasks } = Route.useLoaderData()
  const { project } = Route.useRouteContext()
  const { task } = Route.useSearch()
  const fetchSkills = useCallback(() => listAllSkills(root), [root])
  if (root !== project.path)
    return (
      <div className="flex flex-col gap-6">
        <Header />
        <p role="status">Loading tasks…</p>
      </div>
    )
  return (
    <TasksPage
      key={project.path}
      tasks={tasks}
      initial={task}
      fetchSkills={fetchSkills}
      scope={{ path: "tasks", label: "Project Task" }}
      header={Header}
      api={{
        get: (id) => getTask(root, id),
        save: (originalId, t, renames) => saveTask(root, originalId, t, renames),
        duplicate: (id, newId) => duplicateTask(root, id, newId),
        remove: (id) => removeTask(root, id),
        makeGlobal: (id) => makeTaskGlobal(root, id),
      }}
    />
  )
}
