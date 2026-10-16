import { createFileRoute } from "@tanstack/react-router"
import { SiteHeader } from "@/components/site-header"

export const Route = createFileRoute("/skills/")({
  component: () => <SiteHeader title="Skills" />,
})
