import { type ReactNode } from "react"
import { createFileRoute } from "@tanstack/react-router"
import { PageHeader } from "@/components/page-header"
import { AgentsError, AgentsPage } from "@/components/agents-page"
import {
  duplicateAgent,
  getAgent,
  listAgents,
  removeAgent,
  requireProject,
  saveAgent,
} from "@/lib/projects"
import { loadReferences, usedByFor } from "@/lib/references"

export const Route = createFileRoute("/agents/")({
  beforeLoad: requireProject,
  loader: async ({ context }) => ({
    root: context.project.path,
    agents: await listAgents(context.project.path),
    references: await loadReferences(context.project.path),
  }),
  component: AgentsRoute,
  pendingComponent: () => <p role="status">Loading agents…</p>,
  errorComponent: ({ error }) => <AgentsError error={error} header={Header} />,
})

function Header({ action }: { action?: ReactNode }) {
  return (
    <PageHeader
      title="Agents"
      description="Agent definitions for this project."
      action={action}
    />
  )
}

function AgentsRoute() {
  const { project } = Route.useRouteContext()
  const { root, agents, references } = Route.useLoaderData()
  if (root !== project.path)
    return (
      <div className="flex flex-col gap-6">
        <Header />
        <p role="status">Loading agents…</p>
      </div>
    )
  return (
    <AgentsPage
      key={project.path}
      agents={agents}
      usedBy={usedByFor(references?.agents)}
      root={root}
      header={Header}
      api={{
        get: (id) => getAgent(root, id),
        save: (originalId, agent) => saveAgent(root, originalId, agent),
        duplicate: (id, newId) => duplicateAgent(root, id, newId),
        remove: (id) => removeAgent(root, id),
      }}
    />
  )
}
