import { SearchXIcon, WorkflowIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { NewWorkflowButton } from "./workflows-table"

export function NoWorkflows() {
  return (
    <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed py-16 text-center">
      <WorkflowIcon aria-hidden="true" className="size-6 text-primary" />
      <h2 className="text-lg font-semibold">Create your first workflow</h2>
      <p className="text-sm text-foreground-dim">
        Workflows define steps, approval gates and review loops for this
        project.
      </p>
      <NewWorkflowButton />
    </div>
  )
}

export function NoResults({
  query,
  onClear,
}: {
  query: string
  onClear: () => void
}) {
  return (
    <div className="flex flex-col items-center gap-3 py-16 text-center">
      <SearchXIcon
        aria-hidden="true"
        className="size-6 text-muted-foreground"
      />
      <p className="text-base">No workflows match “{query}”</p>
      <Button variant="outline" size="sm" onClick={onClear}>
        Clear search
      </Button>
    </div>
  )
}
