import { useRef, useState } from "react"
import { createFileRoute, useRouter } from "@tanstack/react-router"
import { Button } from "@/components/ui/button"
import { ListSearch } from "@/components/list-search"
import { PageHeader } from "@/components/page-header"
import { listAgents, requireProject } from "@/lib/projects"
import { AgentsList, filterAgents } from "./-components/agents-list"

export const Route = createFileRoute("/agents/")({
  beforeLoad: requireProject,
  loader: async ({ context }) => ({
    root: context.project.path,
    agents: await listAgents(context.project.path),
  }),
  component: AgentsRoute,
  pendingComponent: () => <p role="status">Loading agents…</p>,
  errorComponent: AgentsError,
})

function Header() {
  return (
    <PageHeader title="Agents" description="Agent definitions for this project." />
  )
}

function AgentsRoute() {
  const { project } = Route.useRouteContext()
  return <AgentsHome key={project.path} />
}

function AgentsHome() {
  const { root, agents } = Route.useLoaderData()
  const { project } = Route.useRouteContext()
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)

  if (root !== project.path)
    return (
      <div className="flex flex-col gap-6">
        <Header />
        <p role="status">Loading agents…</p>
      </div>
    )

  return (
    <div className="flex flex-col gap-4">
      <Header />
      {agents.length > 0 ? (
        <ListSearch
          noun="agent"
          value={query}
          onChange={setQuery}
          shown={filterAgents(agents, query).length}
          total={agents.length}
          inputRef={search}
        />
      ) : null}
      <AgentsList
        agents={agents}
        query={query}
        onClear={() => {
          setQuery("")
          search.current?.focus()
        }}
      />
    </div>
  )
}

function AgentsError({ error }: { error: unknown }) {
  const router = useRouter()
  return (
    <div className="flex flex-col gap-6">
      <Header />
      <div className="flex flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
        <p className="text-sm [overflow-wrap:anywhere]">
          Couldn’t load agents:{" "}
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
