import { type ReactNode } from "react"
import { createFileRoute } from "@tanstack/react-router"
import { PageHeader } from "@/components/page-header"
import { AgentsError, AgentsPage } from "@/components/agents-page"
import {
  duplicateGlobalAgent,
  getGlobalAgent,
  listGlobalAgents,
  removeGlobalAgent,
  saveGlobalAgent,
} from "@/lib/projects"
import { loadReferences, usedByFor } from "@/lib/references"

export const Route = createFileRoute("/global-agents/")({
  loader: async ({ context }) => ({
    agents: await listGlobalAgents(),
    references: await loadReferences(context.project?.path),
  }),
  component: GlobalAgentsRoute,
  pendingComponent: () => <p role="status">Loading global agents…</p>,
  errorComponent: ({ error }) => <AgentsError error={error} header={Header} />,
})

function Header({ action }: { action?: ReactNode }) {
  return (
    <PageHeader
      title="Global agents"
      description="Agents available to every project on this machine. They can only use global skills and MCPs. Stored in ~/.config/unelos."
      action={action}
    />
  )
}

function GlobalAgentsRoute() {
  const { agents, references } = Route.useLoaderData()
  const { project } = Route.useRouteContext()
  const root = project?.path ?? null
  return (
    <AgentsPage
      agents={agents}
      usedBy={usedByFor(references?.agents, { global: true, project: project?.name })}
      root={null}
      global
      header={Header}
      api={{
        get: getGlobalAgent,
        save: (originalId, agent) => saveGlobalAgent(root, originalId, agent),
        duplicate: duplicateGlobalAgent,
        remove: (id) => removeGlobalAgent(root, id),
      }}
    />
  )
}
