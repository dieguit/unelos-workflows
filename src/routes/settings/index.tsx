import { createFileRoute } from "@tanstack/react-router"
import { SiteHeader } from "@/components/site-header"
import { requireProject } from "@/lib/projects"

export const Route = createFileRoute("/settings/")({
  beforeLoad: requireProject,
  component: () => <SiteHeader title="Project settings" />,
})
