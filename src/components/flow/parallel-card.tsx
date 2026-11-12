import { ArrowRightIcon, FileInputIcon, FileOutputIcon, MergeIcon, PlusIcon } from "lucide-react"
import { cn } from "cn"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import type { WorkflowStep } from "@/lib/projects"
import { humanize, workerTitle } from "@/lib/step-edit"
import { cardFrame, IssueBadge, StepIconBox, VisitBadge, type StepStatus } from "./step-card"

function Chip({ icon: Icon, name, note }: { icon: typeof FileInputIcon; name: string; note: string }) {
  return (
    <span className="inline-flex max-w-full items-center gap-1.5 rounded-md bg-muted px-2 py-1 text-xs">
      <Icon className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
      <span className="truncate font-mono">{name}</span>
      <span className="shrink-0 text-muted-foreground">{note}</span>
    </span>
  )
}

export function ParallelCard({ step, issues, highlight, selected, selectedWorker, onSelect, onSelectWorker, onAddTask, status, readOnly }: {
  step: WorkflowStep
  issues: number
  highlight: boolean
  selected?: boolean
  selectedWorker?: number | null
  onSelect?: (id: string) => void
  onSelectWorker?: (index: number) => void
  onAddTask?: () => void
  status?: StepStatus
  readOnly?: boolean
}) {
  const n = step.workers.length
  return (
    <div id={`flow-step-${step.id}`} data-selected={selected} className={cn(cardFrame(issues, highlight, selected && selectedWorker == null, status), "relative w-105 p-3", onSelect && "hover:border-primary/60 has-[>button:focus-visible]:ring-3 has-[>button:focus-visible]:ring-ring/50")}>
      {onSelect && (
        <button
          type="button"
          aria-label={`${readOnly ? "View" : "Edit"} ${humanize(step.id)}`}
          aria-pressed={!!selected && selectedWorker == null}
          onClick={() => onSelect(step.id)}
          className="absolute inset-0 z-1 cursor-pointer rounded-lg outline-none"
        />
      )}
      <div className="flex items-center gap-3">
        <StepIconBox kind="parallel" status={status} />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h3 className="truncate font-semibold">{humanize(step.id)}</h3>
            <IssueBadge count={issues} />
            <VisitBadge status={status} />
          </div>
          <p className="truncate text-sm text-muted-foreground">
            {n} {n === 1 ? "task" : "tasks"} · waits for all
          </p>
        </div>
        {!readOnly && (
          <Button variant="outline" size="xs" className="relative z-2" onClick={onAddTask}>
            <PlusIcon /> Add task
          </Button>
        )}
        <Badge variant="outline" className="rounded-sm border-pending/50 text-pending">Parallel</Badge>
      </div>
      {step.inputs.length > 0 && (
        <div className="mt-2 flex flex-wrap justify-center gap-1.5">
          {step.inputs.map((input) => <Chip key={input} icon={FileInputIcon} name={input} note="shared by all tasks" />)}
        </div>
      )}
      <div aria-hidden="true" className="relative h-4">
        <span className="absolute top-0 left-1/2 h-2 border-l border-border-strong" />
        <span className="absolute top-2 right-1/2 left-2 border-t border-border-strong" />
        <span className="absolute top-2 bottom-0 left-2 border-l border-border-strong" />
      </div>
      <ol aria-label={`${humanize(step.id)} tasks`} className="space-y-2 px-5">
        {step.workers.map((worker, i) => (
          <li key={i} className={cn("relative z-2 flex items-center gap-3 rounded-md border bg-background/40 px-2.5 py-2", onSelectWorker && "hover:border-primary/60 has-[>button:focus-visible]:ring-3 has-[>button:focus-visible]:ring-ring/50", selectedWorker === i && "border-primary bg-selection ring-1 ring-primary")}>
            {onSelectWorker && (
              <button
                type="button"
                aria-label={`Edit ${workerTitle(worker) || `task ${i + 1}`}`}
                aria-pressed={selectedWorker === i}
                onClick={() => onSelectWorker(i)}
                className="absolute inset-0 z-1 cursor-pointer rounded-md outline-none"
              />
            )}
            <span aria-hidden="true" className={cn("absolute -top-px -left-[13px] border-l border-border-strong", i === n - 1 ? "bottom-1/2" : "-bottom-[9px]")} />
            <span aria-hidden="true" className={cn("absolute -right-[13px] -bottom-[9px] border-r border-border-strong", i === 0 ? "top-1/2" : "-top-px")} />
            <span aria-hidden="true" className="absolute top-1/2 -left-[13px] w-[9px] border-t border-border-strong" />
            <span aria-hidden="true" className="absolute top-1/2 -left-1 -translate-y-1/2 border-y-4 border-l-4 border-y-transparent border-l-border-strong" />
            <span aria-hidden="true" className="absolute top-1/2 -right-[13px] w-3 border-t border-border-strong" />
            <StepIconBox kind="task" />
            <div className="min-w-0 flex-1">
              <p className="truncate font-semibold">{workerTitle(worker) || "Untitled task"}</p>
              <p className="truncate text-sm text-muted-foreground">{worker.agent} · subagent</p>
            </div>
            <Badge variant="outline" className="rounded-sm text-muted-foreground">Task {i + 1}</Badge>
          </li>
        ))}
      </ol>
      <div aria-hidden="true" className="relative h-5">
        <span className="absolute top-0 right-2 h-2 border-r border-border-strong" />
        <span className="absolute top-2 right-2 left-1/2 border-t border-border-strong" />
        <span className="absolute top-2 left-1/2 h-1.5 border-l border-border-strong" />
        <span className="absolute bottom-0 left-1/2 -translate-x-[calc(50%-0.5px)] border-x-4 border-t-4 border-x-transparent border-t-border-strong" />
      </div>
      <div className="flex items-center justify-center gap-2">
        <span className="inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium">
          <MergeIcon className="size-3.5" aria-hidden="true" /> Merge
        </span>
        {step.outputs.length > 0 && <ArrowRightIcon className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />}
        <div className="flex min-w-0 flex-wrap gap-1.5">
          {step.outputs.map((output) => <Chip key={output} icon={FileOutputIcon} name={output} note="aggregate" />)}
        </div>
      </div>
    </div>
  )
}
