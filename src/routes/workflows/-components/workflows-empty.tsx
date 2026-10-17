import { SearchXIcon, WorkflowIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { EmptyState } from "@/components/empty-state"
import { NewWorkflowButton } from "./workflows-table"

export function NoWorkflows() {
  return (
    <EmptyState
      dashed
      icon={WorkflowIcon}
      title="Create your first workflow"
      description="Workflows define steps, approval gates and review loops for this project."
      action={<NewWorkflowButton />}
    />
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
    <EmptyState
      icon={SearchXIcon}
      title={`No workflows match “${query}”`}
      action={
        <Button variant="outline" size="sm" onClick={onClear}>
          Clear search
        </Button>
      }
    />
  )
}
