import type { ReactNode } from "react"
import { createFileRoute } from "@tanstack/react-router"
import { z } from "zod"
import { PageHeader } from "@/components/page-header"
import { McpsError, McpsPage } from "@/components/mcps-page"
import {
  getGlobalMcp,
  listGlobalMcps,
  removeGlobalMcp,
  saveGlobalMcp,
} from "@/lib/projects"

export const Route = createFileRoute("/global-mcps/")({
  validateSearch: z.object({ mcp: z.string().optional().catch(undefined) }),
  loader: async () => ({ mcps: await listGlobalMcps() }),
  component: GlobalMcpsRoute,
  pendingComponent: () => <p role="status">Loading global MCPs…</p>,
  errorComponent: ({ error }) => <McpsError error={error} header={Header} />,
})

function Header({ action }: { action?: ReactNode }) {
  return (
    <PageHeader
      title="Global MCPs"
      description="MCP servers available to every project on this machine. Stored in ~/.config/unelos."
      action={action}
    />
  )
}

function GlobalMcpsRoute() {
  const { mcps } = Route.useLoaderData()
  const { mcp } = Route.useSearch()
  const { project } = Route.useRouteContext()
  const root = project?.path ?? null
  return (
    <McpsPage
      mcps={mcps}
      initial={mcp}
      scope={{ path: "global/mcps", label: "Global MCP" }}
      header={Header}
      api={{
        get: getGlobalMcp,
        save: (id, doc) => saveGlobalMcp(root, id, doc),
        remove: (id) => removeGlobalMcp(root, id),
      }}
    />
  )
}
