import { createFileRoute, redirect } from "@tanstack/react-router"
import { NoProject } from "./-components/no-project"

export const Route = createFileRoute("/")({
  beforeLoad: ({ context }) => {
    if (context.project) throw redirect({ to: "/workflows" })
  },
  component: NoProject,
})
