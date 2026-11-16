import type { ReactNode } from "react"
import { createFileRoute } from "@tanstack/react-router"
import { z } from "zod"
import { PageHeader } from "@/components/page-header"
import { loadReferences, usedByFor } from "@/lib/references"
import { McpsError, McpsPage } from "@/components/mcps-page"
import {
  getGlobalMcp,
  listGlobalMcps,
  removeGlobalMcp,
  saveGlobalMcp,
} from "@/lib/projects"

export const Route = createFileRoute("/global-mcps/")({
  validateSearch: z.object({ mcp: z.string().optional().catch(undefined) }),
  loader: async ({ context }) => ({
    mcps: await listGlobalMcps(),
    references: await loadReferences(context.project?.path),
  }),
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
  const { mcps, references } = Route.useLoaderData()
  const { mcp } = Route.useSearch()
  const { project } = Route.useRouteContext()
  const root = project?.path ?? null
  return (
    <McpsPage
      mcps={mcps}
      usedBy={usedByFor(references?.mcps, { global: true, project: project?.name })}
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
