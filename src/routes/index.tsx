import { createFileRoute, redirect } from "@tanstack/react-router"
import { NoProject } from "./-components/no-project"

export const Route = createFileRoute("/")({
  validateSearch: (search: Record<string, unknown>): { picker?: boolean } => ({
    picker: search.picker === true ? true : undefined,
  }),
  beforeLoad: ({ context, search }) => {
    if (context.project && !search.picker) throw redirect({ to: "/workflows" })
  },
  component: NoProject,
})
