import { CircleAlertIcon } from "lucide-react"
import { cn } from "cn"
import { Badge } from "@/components/ui/badge"
import type { WorkflowStep } from "@/lib/projects"
import { humanize, taskLabel } from "../../-lib/step-edit"
import { stepIconColors, stepIcons } from "./step-icon"

export function IssueBadge({ count }: { count: number }) {
  if (!count) return null
  return (
    <Badge variant="destructive" className="rounded-sm">
      <CircleAlertIcon /> {count} {count === 1 ? "issue" : "issues"}
    </Badge>
  )
}

export function StepIconBox({ kind }: { kind: WorkflowStep["kind"] }) {
  const Icon = stepIcons[kind]
  return (
    <span className={cn("flex size-8 shrink-0 items-center justify-center rounded-md bg-muted", stepIconColors[kind])}>
      <Icon className="size-4" aria-hidden="true" />
    </span>
  )
}

export function cardFrame(issues: number, highlight: boolean, selected = false) {
  return cn(
    "scroll-m-24 rounded-lg border bg-card text-card-foreground shadow-sm transition-shadow duration-500",
    issues > 0 && "border-destructive",
    selected && "border-primary bg-selection ring-1 ring-primary",
    highlight && "ring-3 ring-primary/60"
  )
}

export function StepCard({
  step,
  issues,
  highlight,
  selected,
  onSelect,
}: {
  step: WorkflowStep
  issues: number
  highlight: boolean
  selected?: boolean
  onSelect?: (id: string) => void
}) {
  if (step.kind === "end")
    return (
      <div
        id={`flow-step-${step.id}`}
        className={cn(
          cardFrame(issues, highlight),
          "flex items-center gap-2 rounded-full py-1.5 pr-4 pl-1.5"
        )}
      >
        <StepIconBox kind="end" />
        <span className="font-semibold">{humanize(step.id)}</span>
        <span className="text-xs text-muted-foreground">End</span>
        <IssueBadge count={issues} />
      </div>
    )
  const subtitle =
    step.kind === "human-gate"
      ? step.inputs[0]
        ? `Decision on ${step.inputs[0]}`
        : "Human decision"
      : [step.title, step.task?.source === "named" && step.task.id && taskLabel(step.task.id), step.agent].filter(Boolean).join(" · ") ||
        "Not configured"
  const badge =
    step.kind === "human-gate"
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
        cardFrame(issues, highlight, selected),
        "relative flex w-80 items-center gap-3 px-3 py-2.5",
        onSelect &&
          "hover:border-primary/60 has-[>button:focus-visible]:ring-3 has-[>button:focus-visible]:ring-ring/50"
      )}
    >
      {onSelect && (
        // Covers the whole card so it stays one click target while the content keeps its semantics.
        <button
          type="button"
          aria-label={`Edit ${humanize(step.id)}`}
          aria-pressed={!!selected}
          onClick={() => onSelect(step.id)}
          className="absolute inset-0 z-1 cursor-pointer rounded-lg outline-none"
        />
      )}
      <StepIconBox kind={step.kind} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <h3 className="truncate font-semibold">{humanize(step.id)}</h3>
          <IssueBadge count={issues} />
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
