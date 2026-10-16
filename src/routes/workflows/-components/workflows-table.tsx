import {
  EllipsisIcon,
  PlusIcon,
  TriangleAlertIcon,
  CircleCheckIcon,
} from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
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

function Truncated({ text, className }: { text: string; className?: string }) {
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <span
            tabIndex={0}
            className={`block truncate outline-offset-2 focus-visible:outline-2 focus-visible:outline-ring ${className ?? ""}`}
          />
        }
      >
        {text}
      </TooltipTrigger>
      <TooltipContent className="max-w-sm">{text}</TooltipContent>
    </Tooltip>
  )
}

function Definition({ problem }: { problem: string | null }) {
  if (!problem)
    return (
      <Badge className="bg-success-muted text-success">
        <CircleCheckIcon /> Valid
      </Badge>
    )
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <Badge
            tabIndex={0}
            className="max-w-full bg-warning-muted text-warning outline-offset-2 focus-visible:outline-2 focus-visible:outline-ring"
          />
        }
      >
        <TriangleAlertIcon />
        <span className="truncate">{problem}</span>
      </TooltipTrigger>
      <TooltipContent className="max-w-sm">{problem}</TooltipContent>
    </Tooltip>
  )
}

function moveFocus(e: React.KeyboardEvent<HTMLTableSectionElement>) {
  if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return
  const rows = [
    ...e.currentTarget.querySelectorAll<HTMLElement>("tr[data-row]"),
  ]
  const i = rows.indexOf(document.activeElement as HTMLElement)
  if (i === -1) return
  e.preventDefault()
  rows[
    Math.max(0, Math.min(rows.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)))
  ].focus()
}

const th =
  "sticky -top-6 h-auto bg-muted px-2 py-2 text-xs font-medium tracking-[0.6px] text-muted-foreground shadow-[inset_0_-1px_0_var(--border)]"
const td = "px-2 py-2.5"

export function WorkflowsTable({
  workflows,
}: {
  workflows: WorkflowSummary[]
}) {
  return (
    <div className="overflow-clip rounded-lg border bg-card">
      <Table className="table-fixed">
        <TableHeader className="sticky -top-6 z-10 bg-muted">
          <TableRow className="hover:bg-transparent">
            <TableHead className={`${th} w-70 pl-3`}>NAME</TableHead>
            <TableHead className={th}>DESCRIPTION</TableHead>
            <TableHead className={`${th} w-18 text-right`}>STEPS</TableHead>
            <TableHead className={`${th} w-62`}>DEFINITION</TableHead>
            <TableHead className={`${th} w-12 pr-3`}>
              <span className="sr-only">Actions</span>
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody onKeyDown={moveFocus}>
          {workflows.map((w) => (
            <TableRow
              key={w.file}
              data-row
              tabIndex={0}
              className="outline-none hover:bg-accent focus-visible:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
            >
              <TableCell className={`${td} pl-3`}>
                <span className="block truncate text-base font-medium">
                  {w.name}
                </span>
                <span className="block truncate font-mono text-xs text-muted-foreground">
                  {w.file}
                </span>
              </TableCell>
              <TableCell className={`${td} text-foreground-dim`}>
                {w.description ? <Truncated text={w.description} /> : null}
              </TableCell>
              <TableCell
                className={`${td} text-right text-foreground-dim tabular-nums`}
              >
                {w.steps}
              </TableCell>
              <TableCell className={td}>
                <Definition problem={w.problem} />
              </TableCell>
              <TableCell
                className={`${td} pr-3`}
                onKeyDown={(e) => e.stopPropagation()}
              >
                <DropdownMenu>
                  <DropdownMenuTrigger
                    render={
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={`Actions for ${w.name}`}
                      />
                    }
                  >
                    <EllipsisIcon />
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem disabled>
                      Open<DropdownMenuShortcut>↵</DropdownMenuShortcut>
                    </DropdownMenuItem>
                    <DropdownMenuItem disabled>
                      Duplicate…<DropdownMenuShortcut>⌘D</DropdownMenuShortcut>
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}
