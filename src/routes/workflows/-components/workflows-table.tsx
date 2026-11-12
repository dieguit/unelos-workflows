import { PlusIcon } from "lucide-react"
import type { ReactNode } from "react"
import { Button } from "@/components/ui/button"
import {
  ListBody,
  ListCell,
  ListHead,
  ListHeader,
  ListPrimary,
  ListRow,
  ListTable,
  Truncated,
} from "@/components/list-table"
import type { WorkflowSummary } from "@/lib/projects"

export function NewWorkflowButton({ onClick, disabled }: { onClick?: () => void; disabled?: boolean }) {
  return (
    <Button data-new-workflow disabled={disabled || !onClick} onClick={onClick}>
      <PlusIcon /> New workflow
    </Button>
  )
}

export function WorkflowsTable({
  workflows,
  onOpen,
  actions,
}: {
  workflows: WorkflowSummary[]
  onOpen: (file: string) => void
  actions: (workflow: WorkflowSummary) => ReactNode
}) {
  return (
    <ListTable>
      <ListHeader>
        <ListHead className="w-70">Name</ListHead>
        <ListHead>Description</ListHead>
        <ListHead className="w-18 text-right">Steps</ListHead>
        <ListHead className="w-14">
          <span className="sr-only">Actions</span>
        </ListHead>
      </ListHeader>
      <ListBody>
        {workflows.map((w) => (
          <ListRow
            key={w.file}
            className="cursor-pointer"
            onClick={() => onOpen(w.file)}
            onKeyDown={(e) => {
              if (
                e.target === e.currentTarget &&
                (e.key === "Enter" || e.key === " ")
              ) {
                e.preventDefault()
                onOpen(w.file)
              }
            }}
          >
            <ListCell>
              <ListPrimary
                title={w.name}
                subtitle={w.file}
                problem={w.problem}
              />
            </ListCell>
            <ListCell className="text-foreground-dim">
              {w.description ? <Truncated text={w.description} /> : "—"}
            </ListCell>
            <ListCell className="text-right text-foreground-dim tabular-nums">
              {w.steps ?? "—"}
            </ListCell>
            <ListCell>{actions(w)}</ListCell>
          </ListRow>
        ))}
      </ListBody>
    </ListTable>
  )
}
