import type { ComponentProps, KeyboardEvent, ReactNode } from "react"
import { TriangleAlertIcon } from "lucide-react"
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
import { cn } from "@/lib/utils"

export function ListTable({ children }: { children: ReactNode }) {
  return (
    <div className="overflow-clip rounded-lg border bg-card">
      <Table className="table-fixed">{children}</Table>
    </div>
  )
}

export function ListHeader({ children }: { children: ReactNode }) {
  return (
    <TableHeader className="sticky -top-6 z-10 bg-muted">
      <TableRow className="hover:bg-transparent">{children}</TableRow>
    </TableHeader>
  )
}

export function ListHead({ className, ...props }: ComponentProps<"th">) {
  return (
    <TableHead
      scope="col"
      className={cn(
        "sticky -top-6 h-auto bg-muted px-2 py-2 text-xs font-medium tracking-[0.6px] text-muted-foreground uppercase shadow-[inset_0_-1px_0_var(--border)] first:pl-3 last:pr-3",
        className
      )}
      {...props}
    />
  )
}

function moveFocus(e: KeyboardEvent<HTMLTableSectionElement>) {
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

export function ListBody({ children }: { children: ReactNode }) {
  return <TableBody onKeyDown={moveFocus}>{children}</TableBody>
}

export function ListRow({ className, ...props }: ComponentProps<"tr">) {
  return (
    <TableRow
      data-row
      tabIndex={0}
      className={cn(
        "outline-none hover:bg-accent focus-visible:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset",
        className
      )}
      {...props}
    />
  )
}

export function ListCell({ className, ...props }: ComponentProps<"td">) {
  return (
    <TableCell
      className={cn("px-2 py-2.5 first:pl-3 last:pr-3", className)}
      {...props}
    />
  )
}

function Problem({ problem }: { problem: string }) {
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <span
            tabIndex={0}
            role="img"
            aria-label={`Problem: ${problem}`}
            className="shrink-0 rounded-sm text-warning outline-offset-2 focus-visible:outline-2 focus-visible:outline-ring"
          />
        }
      >
        <TriangleAlertIcon aria-hidden="true" className="size-4" />
      </TooltipTrigger>
      <TooltipContent className="max-w-sm [overflow-wrap:anywhere]">
        {problem}
      </TooltipContent>
    </Tooltip>
  )
}

export function ListPrimary({
  title,
  subtitle,
  problem,
  badge,
}: {
  title: string
  subtitle?: string
  problem?: string | null
  badge?: ReactNode
}) {
  return (
    <>
      <span className="flex min-w-0 items-center gap-1.5">
        <span className="truncate text-base font-medium">{title}</span>
        {badge}
        {problem ? <Problem problem={problem} /> : null}
      </span>
      {subtitle ? (
        <span className="block truncate font-mono text-xs text-muted-foreground">
          {subtitle}
        </span>
      ) : null}
    </>
  )
}

export function Truncated({
  text,
  className,
}: {
  text: string
  className?: string
}) {
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <span
            tabIndex={0}
            className={cn(
              "block truncate outline-offset-2 focus-visible:outline-2 focus-visible:outline-ring",
              className
            )}
          />
        }
      >
        {text}
      </TooltipTrigger>
      <TooltipContent className="max-w-sm">{text}</TooltipContent>
    </Tooltip>
  )
}
