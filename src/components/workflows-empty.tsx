import { SearchXIcon, WorkflowIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { EmptyState } from "@/components/empty-state"
import { ImportWorkflowButton, NewWorkflowButton } from "./workflows-table"

export function NoWorkflows({ onCreate, creating, onImport, description }: { onCreate: () => void; creating: boolean; onImport?: () => void; description?: string }) {
  return (
    <EmptyState
      dashed
      icon={WorkflowIcon}
      title="No workflows yet"
      description={description ?? "Workflows define steps, approval gates and review loops for this project. Create one from scratch, or import one from a folder or git repository."}
      action={
        <>
          <div className="flex gap-2">
            {onImport && <ImportWorkflowButton onClick={onImport} label="Import workflow" />}
            <NewWorkflowButton onClick={onCreate} disabled={creating} />
          </div>
          {onImport && (
            <p className="text-xs text-muted-foreground">
              An import brings in one workflow with its agents, tasks, skills and MCP servers.
            </p>
          )}
        </>
      }
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
