import {
  ChevronRightIcon,
  CopyIcon,
  PlusIcon,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
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

export function NewWorkflowButton() {
  return (
    <Tooltip>
      <TooltipTrigger render={<span tabIndex={0} />}>
        <Button data-new-workflow disabled>
          <PlusIcon /> New workflow
        </Button>
      </TooltipTrigger>
      <TooltipContent>Coming soon</TooltipContent>
    </Tooltip>
  )
}

export function WorkflowsTable({
  workflows,
}: {
  workflows: WorkflowSummary[]
}) {
  return (
    <ListTable>
      <ListHeader>
        <ListHead className="w-70">Name</ListHead>
        <ListHead>Description</ListHead>
        <ListHead className="w-18 text-right">Steps</ListHead>
        <ListHead className="w-32">
          <span className="sr-only">Actions</span>
        </ListHead>
      </ListHeader>
      <ListBody>
        {workflows.map((w) => (
          <ListRow key={w.file}>
            <ListCell>
              <ListPrimary title={w.name} subtitle={w.file} problem={w.problem} />
            </ListCell>
            <ListCell className="text-foreground-dim">
              {w.description ? <Truncated text={w.description} /> : "—"}
            </ListCell>
            <ListCell className="text-right text-foreground-dim tabular-nums">
              {w.steps ?? "—"}
            </ListCell>
            <ListCell onKeyDown={(e) => e.stopPropagation()}>
              <div className="flex items-center justify-end gap-1">
                <Button
                  variant="outline"
                  size="icon-sm"
                  className="disabled:pointer-events-auto hover:border-ring hover:bg-accent"
                  aria-label={`Duplicate ${w.name}`}
                  disabled
                >
                  <CopyIcon />
                </Button>
                <Button variant="ghost" size="sm" disabled>
                  Open <ChevronRightIcon data-icon="inline-end" />
                </Button>
              </div>
            </ListCell>
          </ListRow>
        ))}
      </ListBody>
    </ListTable>
  )
}
