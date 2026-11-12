import type { ReactNode } from "react"
import { HourglassIcon, Loader2Icon } from "lucide-react"
import { Badge } from "@/components/ui/badge"
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
import { humanize } from "@/lib/step-edit"
import { formatTime } from "../-lib/format"
import type { RunListItem } from "../-lib/runs-api"

function StatusBadge({ status }: { status: RunListItem["status"] }) {
  if (status === "waiting")
    return (
      <Badge variant="outline" className="rounded-sm border-warning/50 text-warning">
        <HourglassIcon aria-hidden="true" /> Waiting
      </Badge>
    )
  return (
    <Badge variant="outline" className="rounded-sm border-primary/50 text-primary">
      <Loader2Icon aria-hidden="true" className="animate-spin" /> Running
    </Badge>
  )
}

export function RunsTable({
  runs,
  finished,
  onOpen,
  actions,
}: {
  runs: RunListItem[]
  finished?: boolean
  onOpen: (runId: string) => void
  actions: (run: RunListItem) => ReactNode
}) {
  return (
    <ListTable>
      <ListHeader>
        <ListHead className="w-70">Name</ListHead>
        <ListHead>Workflow</ListHead>
        {!finished && <ListHead>Current step</ListHead>}
        {!finished && <ListHead className="w-28">Status</ListHead>}
        <ListHead className="w-40">Started</ListHead>
        <ListHead className="w-40">{finished ? "Finished" : "Updated"}</ListHead>
        <ListHead className="w-14">
          <span className="sr-only">Actions</span>
        </ListHead>
      </ListHeader>
      <ListBody>
        {runs.map((r) => (
          <ListRow
            key={r.runId}
            className="cursor-pointer"
            onClick={() => onOpen(r.runId)}
            onKeyDown={(e) => {
              if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) {
                e.preventDefault()
                onOpen(r.runId)
              }
            }}
          >
            <ListCell>
              <ListPrimary title={r.name} subtitle={r.runId} />
            </ListCell>
            <ListCell className="text-foreground-dim">
              <Truncated text={r.workflowName} />
            </ListCell>
            {!finished && (
              <ListCell className="text-foreground-dim">
                <Truncated text={humanize(r.nodeId)} />
              </ListCell>
            )}
            {!finished && (
              <ListCell>
                <StatusBadge status={r.status} />
              </ListCell>
            )}
            <ListCell className="text-foreground-dim">{formatTime(r.startedAt)}</ListCell>
            <ListCell className="text-foreground-dim">{formatTime(r.updatedAt)}</ListCell>
            <ListCell>{actions(r)}</ListCell>
          </ListRow>
        ))}
      </ListBody>
    </ListTable>
  )
}
