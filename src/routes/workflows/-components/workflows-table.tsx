import { DownloadIcon, PlusIcon } from "lucide-react"
import { useEffect, type ReactNode } from "react"
import { cn } from "@/lib/utils"
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

export function ImportWorkflowButton({ onClick, label = "Import" }: { onClick?: () => void; label?: string }) {
  return (
    <Button variant="secondary" disabled={!onClick} onClick={onClick}>
      <DownloadIcon /> {label}
    </Button>
  )
}

export function WorkflowsTable({
  workflows,
  onOpen,
  actions,
  highlight,
}: {
  workflows: WorkflowSummary[]
  onOpen: (file: string) => void
  actions: (workflow: WorkflowSummary) => ReactNode
  highlight?: string
}) {
  useEffect(() => {
    if (highlight) document.querySelector(`[data-file="${CSS.escape(highlight)}"]`)?.scrollIntoView({ block: "nearest" })
  }, [highlight])
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
            data-file={w.file}
            className={cn("cursor-pointer transition-colors duration-700", w.file === highlight && "bg-primary-muted")}
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
