import { FileTextIcon, Loader2Icon } from "lucide-react"
import { cn } from "cn"
import { Badge } from "@/components/ui/badge"
import { StepIconBox } from "@/components/flow/step-card"
import { PanelHeader } from "@/components/step-panel/panel-chrome"
import { rowFrame, Section } from "@/components/step-panel/panel-ui"
import { humanize } from "@/lib/step-edit"
import type { Execution } from "../../-lib/run-graph"
import type { ArtifactReference } from "../../-lib/runs-api"

function ArtifactList({
  label,
  items,
  open,
  onOpen,
  existingArtifactPaths,
}: {
  label: string
  items: { name: string; path: string }[]
  open?: string
  onOpen: (artifact: ArtifactReference) => void
  existingArtifactPaths?: string[]
}) {
  if (!items.length) return null
  return (
    <Section label={label}>
      <ul className="flex flex-col gap-1.5">
        {items.map((a) => {
          const available =
            existingArtifactPaths === undefined ||
            existingArtifactPaths.includes(a.path)
          return (
            <li key={`${a.name}:${a.path}`}>
              {available ? (
                <button
                  type="button"
                  title={a.path}
                  aria-pressed={open === a.path}
                  onClick={() => onOpen({ name: a.name, path: a.path })}
                  className={cn(
                    rowFrame,
                    "w-full cursor-pointer pr-2.5 text-left outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring",
                    open === a.path && "border-primary bg-selection"
                  )}
                >
                  <FileTextIcon
                    aria-hidden="true"
                    className="size-3.5 shrink-0 text-muted-foreground"
                  />
                  <span className="truncate font-mono">{a.name}</span>
                </button>
              ) : (
                <span
                  title={a.path}
                  className={cn(
                    rowFrame,
                    "w-full pr-2.5 text-left text-muted-foreground"
                  )}
                >
                  <FileTextIcon
                    aria-hidden="true"
                    className="size-3.5 shrink-0"
                  />
                  <span className="truncate font-mono">{a.name}</span>
                  <span className="ml-auto shrink-0 text-xs">Unavailable</span>
                </span>
              )}
            </li>
          )
        })}
      </ul>
    </Section>
  )
}

function ExecutionView({
  execution,
  open,
  onOpen,
  existingArtifactPaths,
}: {
  execution: Execution
  open?: string
  onOpen: (artifact: ArtifactReference) => void
  existingArtifactPaths?: string[]
}) {
  const { record, outcome, reason, active, n } = execution
  const { feedback } = record
  return (
    <article
      aria-label={`Execution ${n}`}
      className="flex flex-col gap-3 rounded-lg border p-3"
    >
      <header className="flex items-center gap-2">
        <h3 className="font-semibold">#{n}</h3>
        {active ? (
          <Badge
            variant="outline"
            className="rounded-sm border-primary/50 text-primary"
          >
            <Loader2Icon aria-hidden="true" className="animate-spin" /> In
            progress
          </Badge>
        ) : outcome ? (
          <Badge variant="outline" className="rounded-sm font-mono">
            {outcome}
          </Badge>
        ) : null}
        <span className="ml-auto truncate font-mono text-xs text-muted-foreground">
          {record.executionId}
        </span>
      </header>
      {reason && (
        <p className="text-sm [overflow-wrap:anywhere] text-foreground-dim">
          {reason}
        </p>
      )}
      <ArtifactList
        label="Inputs"
        items={record.inputs}
        open={open}
        onOpen={onOpen}
        existingArtifactPaths={existingArtifactPaths}
      />
      <ArtifactList
        label="Outputs"
        items={record.outputs}
        open={open}
        onOpen={onOpen}
        existingArtifactPaths={existingArtifactPaths}
      />
      <ArtifactList
        label="Worker reports"
        items={record.workerReports.map((w) => ({
          name: w.key,
          path: w.report.path,
        }))}
        open={open}
        onOpen={onOpen}
        existingArtifactPaths={existingArtifactPaths}
      />
      {feedback && (
        <>
          <ArtifactList
            label="Feedback"
            items={feedback.artifacts}
            open={open}
            onOpen={onOpen}
            existingArtifactPaths={existingArtifactPaths}
          />
          {(feedback.outcome ||
            feedback.reason ||
            !feedback.artifacts.length) && (
            <p className="text-xs [overflow-wrap:anywhere] text-muted-foreground">
              From {humanize(feedback.sourceNodeId)}
              {feedback.outcome && (
                <>
                  {" "}
                  · <span className="font-mono">{feedback.outcome}</span>
                </>
              )}
              {feedback.reason && <> · {feedback.reason}</>}
            </p>
          )}
        </>
      )}
    </article>
  )
}

export function ExecutionPanel({
  nodeId,
  kind,
  executions,
  open,
  onOpen,
  onClose,
  existingArtifactPaths,
}: {
  nodeId: string
  kind: "task" | "parallel" | "human-gate" | "end"
  executions: Execution[]
  open?: string
  onOpen: (artifact: ArtifactReference) => void
  onClose: () => void
  existingArtifactPaths?: string[]
}) {
  return (
    <>
      <PanelHeader
        icon={<StepIconBox kind={kind} />}
        title={humanize(nodeId)}
        subtitle={nodeId}
        tag={`${executions.length} ${executions.length === 1 ? "execution" : "executions"}`}
        onClose={onClose}
      />
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4 pt-1">
        {executions.length === 0 && (
          <p className="text-sm text-muted-foreground">
            This step has not run yet.
          </p>
        )}
        {[...executions].reverse().map((e) => (
          <ExecutionView
            key={e.record.executionId}
            execution={e}
            open={open}
            onOpen={onOpen}
            existingArtifactPaths={existingArtifactPaths}
          />
        ))}
      </div>
    </>
  )
}
