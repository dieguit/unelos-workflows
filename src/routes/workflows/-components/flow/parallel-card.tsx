import { ArrowRightIcon, FileInputIcon, FileOutputIcon, MergeIcon, PlusIcon } from "lucide-react"
import { cn } from "cn"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import type { WorkflowStep } from "@/lib/projects"
import { humanize } from "../../-lib/step-edit"
import { cardFrame, IssueBadge, StepIconBox } from "./step-card"

function Chip({ icon: Icon, name, note }: { icon: typeof FileInputIcon; name: string; note: string }) {
  return (
    <span className="inline-flex max-w-full items-center gap-1.5 rounded-md bg-muted px-2 py-1 text-xs">
      <Icon className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
      <span className="truncate font-mono">{name}</span>
      <span className="shrink-0 text-muted-foreground">{note}</span>
    </span>
  )
}

export function ParallelCard({ step, issues, highlight }: { step: WorkflowStep; issues: number; highlight: boolean }) {
  const n = step.workers.length
  return (
    <div id={`flow-step-${step.id}`} className={cn(cardFrame(issues, highlight), "w-105 p-3")}>
      <div className="flex items-center gap-3">
        <StepIconBox kind="parallel" />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h3 className="truncate font-semibold">{humanize(step.id)}</h3>
            <IssueBadge count={issues} />
          </div>
          <p className="truncate text-sm text-muted-foreground">
            {n} {n === 1 ? "task" : "tasks"} · waits for all
          </p>
        </div>
        <Button variant="outline" size="xs" disabled title="Adding tasks is not available yet">
          <PlusIcon /> Add task
        </Button>
        <Badge variant="outline" className="rounded-sm border-pending/50 text-pending">Parallel</Badge>
      </div>
      {step.inputs.length > 0 && (
        <div className="mt-2 flex flex-wrap justify-center gap-1.5">
          {step.inputs.map((input) => <Chip key={input} icon={FileInputIcon} name={input} note="shared by all tasks" />)}
        </div>
      )}
      <ol aria-label={`${humanize(step.id)} tasks`} className="relative mt-2 space-y-2 px-5">
        {/* Split rail (left) and merge rail (right); positions are fixed. */}
        <span aria-hidden="true" className="absolute top-0 bottom-6 left-2 border-l border-border-strong" />
        <span aria-hidden="true" className="absolute top-6 right-2 -bottom-2 border-r border-border-strong" />
        {step.workers.map((worker, i) => (
          <li key={i} className="relative flex items-center gap-3 rounded-md border bg-background/40 px-2.5 py-2">
            <span aria-hidden="true" className="absolute top-1/2 -left-3 w-3 border-t border-border-strong" />
            <span aria-hidden="true" className="absolute top-1/2 -right-3 w-3 border-t border-border-strong" />
            <StepIconBox kind="task" />
            <div className="min-w-0 flex-1">
              <p className="truncate font-semibold">{worker.title}</p>
              <p className="truncate text-sm text-muted-foreground">{worker.agent} · subagent</p>
            </div>
            <Badge variant="outline" className="rounded-sm text-muted-foreground">Task {i + 1}</Badge>
          </li>
        ))}
      </ol>
      <div className="mt-3 flex items-center gap-2">
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
