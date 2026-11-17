import { CheckIcon, CircleAlertIcon, HourglassIcon, Loader2Icon } from "lucide-react"
import { cn } from "cn"
import { Badge } from "@/components/ui/badge"
import type { WorkflowStep } from "@/lib/projects"
import { humanize, taskLabel } from "@/lib/step-edit"
import { stepIconColors, stepIcons } from "./step-icon"

export function IssueBadge({ count }: { count: number }) {
  if (!count) return null
  return (
    <Badge variant="destructive" className="rounded-sm">
      <CircleAlertIcon /> {count} {count === 1 ? "issue" : "issues"}
    </Badge>
  )
}

/** Run progress for one step; only set when the flow is shown for a run. */
export type StepStatus = { state: "current" | "waiting" | "done" | "idle"; visits: number }

const statusLabels = { current: "Running", waiting: "Waiting for a decision", done: "Done", idle: "Not reached" }

export function StepIconBox({ kind, status }: { kind: WorkflowStep["kind"]; status?: StepStatus }) {
  const state = status?.state
  const Icon =
    state === "current" ? Loader2Icon : state === "waiting" ? HourglassIcon : state === "done" ? CheckIcon : stepIcons[kind]
  const color =
    state === "current" ? "text-primary" : state === "waiting" ? "text-warning" : state === "done" ? "text-success" : stepIconColors[kind]
  return (
    <span className={cn("flex size-8 shrink-0 items-center justify-center rounded-md bg-muted", color)}>
      <Icon className={cn("size-4", state === "current" && "animate-spin")} aria-hidden="true" />
      {state && <span className="sr-only">{statusLabels[state]}</span>}
    </span>
  )
}

export function VisitBadge({ status }: { status?: StepStatus }) {
  if (!status || status.visits < 2) return null
  return (
    <Badge variant="outline" className="rounded-sm text-muted-foreground" title={`Visited ${status.visits} times`}>
      ×{status.visits}
    </Badge>
  )
}

export function cardFrame(issues: number, highlight: boolean, selected = false, status?: StepStatus) {
  return cn(
    "scroll-m-24 rounded-lg border bg-card text-card-foreground shadow-sm transition-shadow duration-500",
    issues > 0 && "border-destructive",
    selected && "border-primary bg-selection ring-1 ring-primary",
    highlight && "ring-3 ring-primary/60",
    status?.state === "current" && !selected && "border-primary bg-primary-muted ring-2 ring-primary/40",
    status?.state === "waiting" && !selected && "border-warning bg-warning-muted ring-2 ring-warning/40",
    status?.state === "idle" && "opacity-55"
  )
}

export function StepCard({
  step,
  issues,
  highlight,
  selected,
  onSelect,
  status,
  readOnly,
}: {
  step: WorkflowStep
  issues: number
  highlight: boolean
  selected?: boolean
  onSelect?: (id: string) => void
  status?: StepStatus
  readOnly?: boolean
}) {
  if (step.kind === "end")
    return (
      <div
        id={`flow-step-${step.id}`}
        className={cn(
          cardFrame(issues, highlight, false, status),
          "flex items-center gap-2 rounded-full py-1.5 pr-4 pl-1.5"
        )}
      >
        <StepIconBox kind="end" status={status} />
        <span className="font-semibold">{humanize(step.id)}</span>
        <span className="text-xs text-muted-foreground">End</span>
        <IssueBadge count={issues} />
      </div>
    )
  const subtitle =
    step.kind === "workflow"
      ? `calls ${step.call?.workflow ?? ""}`
      : step.kind === "human-gate"
        ? step.inputs[0]
          ? `Decision on ${step.inputs[0]}`
          : "Human decision"
        : [step.title, step.task?.source === "named" && step.task.id && taskLabel(step.task.id), step.agent].filter(Boolean).join(" · ") ||
          "Not configured"
  const badge =
    step.kind === "workflow"
      ? "Workflow"
      : step.kind === "human-gate"
        ? "Human gate"
        : step.execution === "main"
          ? "Main session"
          : step.execution === "subagent"
            ? "Subagent"
            : null
  return (
    <div
      id={`flow-step-${step.id}`}
      className={cn(
        cardFrame(issues, highlight, selected, status),
        "relative flex w-80 items-center gap-3 px-3 py-2.5",
        onSelect &&
          "hover:border-primary/60 has-[>button:focus-visible]:ring-3 has-[>button:focus-visible]:ring-ring/50"
      )}
    >
      {onSelect && (
        // Covers the whole card so it stays one click target while the content keeps its semantics.
        <button
          type="button"
          aria-label={`${readOnly ? "View" : "Edit"} ${humanize(step.id)}`}
          aria-pressed={!!selected}
          onClick={() => onSelect(step.id)}
          className="absolute inset-0 z-1 cursor-pointer rounded-lg outline-none"
        />
      )}
      <StepIconBox kind={step.kind} status={status} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <h3 className="truncate font-semibold">{humanize(step.id)}</h3>
          <IssueBadge count={issues} />
          <VisitBadge status={status} />
        </div>
        {subtitle && (
          <p className="truncate text-sm text-muted-foreground">{subtitle}</p>
        )}
      </div>
      {badge && (
        <Badge variant="outline" className="rounded-sm text-muted-foreground">
          {badge}
        </Badge>
      )}
    </div>
  )
}
