import type { ReactNode } from "react"
import { createFileRoute } from "@tanstack/react-router"
import { z } from "zod"
import { PageHeader } from "@/components/page-header"
import { McpsError, McpsPage } from "@/components/mcps-page"
import {
  getMcp,
  listMcps,
  removeMcp,
  requireProject,
  saveMcp,
} from "@/lib/projects"

export const Route = createFileRoute("/mcps/")({
  validateSearch: z.object({ mcp: z.string().optional().catch(undefined) }),
  beforeLoad: requireProject,
  loader: async ({ context }) => ({
    root: context.project.path,
    mcps: await listMcps(context.project.path),
  }),
  component: McpsRoute,
  pendingComponent: () => <p role="status">Loading MCPs…</p>,
  errorComponent: ({ error }) => <McpsError error={error} header={Header} />,
})

function Header({ action }: { action?: ReactNode }) {
  return (
    <PageHeader
      title="MCPs"
      description="MCP servers for this project’s agents."
      action={action}
    />
  )
}

function McpsRoute() {
  const { root, mcps } = Route.useLoaderData()
  const { project } = Route.useRouteContext()
  const { mcp } = Route.useSearch()
  if (root !== project.path) return <p role="status">Loading MCPs…</p>
  return (
    <McpsPage
      key={root}
      mcps={mcps}
      initial={mcp}
      scope={{ path: "mcps", label: "Project MCP" }}
      header={Header}
      api={{
        get: (id) => getMcp(root, id),
        save: (id, doc) => saveMcp(root, id, doc),
        remove: (id) => removeMcp(root, id),
      }}
    />
  )
}
