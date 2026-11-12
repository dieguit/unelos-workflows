import { useEffect, useMemo, useState } from "react"
import { Link, useNavigate } from "@tanstack/react-router"
import {
  ArrowLeftIcon,
  HourglassIcon,
  Loader2Icon,
  TriangleAlertIcon,
  CheckIcon,
} from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { FlowPane } from "@/components/flow/flow-pane"
import { FlowToolbar } from "@/components/flow/flow-toolbar"
import { humanize } from "@/lib/step-edit"
import { getWorkflow, type WorkflowDocument } from "@/lib/projects"
import {
  executionsByNode,
  missingNodes,
  nodeStatus,
} from "../../-lib/run-graph"
import {
  deleteRun,
  type ArtifactReference,
  type RunState,
} from "../../-lib/runs-api"
import { ArtifactViewer } from "./artifact-viewer"
import { DeleteRunButton } from "./delete-run-button"
import { ExecutionPanel } from "./execution-panel"

function RunStatus({ current }: { current: RunState["current"] }) {
  if (current.kind === "finished")
    return (
      <Badge
        variant="outline"
        className="rounded-sm border-success/50 text-success"
      >
        <CheckIcon aria-hidden="true" /> Finished
      </Badge>
    )
  if (current.kind === "human-gate")
    return (
      <Badge
        variant="outline"
        className="rounded-sm border-warning/50 text-warning"
      >
        <HourglassIcon aria-hidden="true" /> Waiting
      </Badge>
    )
  return (
    <Badge
      variant="outline"
      className="rounded-sm border-primary/50 text-primary"
    >
      <Loader2Icon aria-hidden="true" className="animate-spin" /> Running
    </Badge>
  )
}

/** Workflow is loaded once; `undefined` while loading, `null` when it cannot be read. */
function useWorkflow(root: string, name: string) {
  const [state, setState] = useState<{
    key: string
    doc: WorkflowDocument | null
  }>()
  const key = `${root}:${name}`
  useEffect(() => {
    let active = true
    getWorkflow(root, name).then(
      (doc) => active && setState({ key, doc }),
      () => active && setState({ key, doc: null })
    )
    return () => {
      active = false
    }
  }, [root, name, key])
  return state?.key === key ? state.doc : undefined
}

export function RunView({ root, run }: { root: string; run: RunState }) {
  const navigate = useNavigate()
  const workflow = useWorkflow(root, run.workflowName)
  const [selected, setSelected] = useState<string>()
  const [artifact, setArtifact] = useState<ArtifactReference>()
  const executions = useMemo(() => executionsByNode(run), [run])
  const stepIds = useMemo(
    () => workflow?.steps.map((s) => s.id) ?? [],
    [workflow]
  )
  const status = useMemo(() => nodeStatus(run, stepIds), [run, stepIds])
  const missing = workflow ? missingNodes(run, stepIds) : []
  const kind = workflow?.steps.find((s) => s.id === selected)?.kind ?? "task"
  const select = (id: string) => {
    setSelected(id)
    setArtifact(undefined)
  }
  return (
    <div className="-mx-8 -mt-6 -mb-8 flex h-[calc(100%+3.5rem)] min-h-0 flex-col">
      <header className="flex min-h-14 shrink-0 items-center gap-3 overflow-x-auto border-b px-3 py-2">
        <Link
          to="/runs"
          className="inline-flex h-7 shrink-0 items-center gap-1 rounded-md px-2.5 text-sm hover:bg-accent"
        >
          <ArrowLeftIcon className="size-4" aria-hidden="true" /> Runs
        </Link>
        <div className="h-6 border-l" />
        <div className="flex min-w-0 flex-col">
          <div className="flex min-w-0 items-baseline gap-2">
            <h1 className="truncate text-lg font-semibold">{run.name}</h1>
            <span className="truncate font-mono text-xs text-muted-foreground">
              {run.workflowName}
            </span>
          </div>
          {run.request && (
            <p
              className="max-w-3xl truncate text-xs text-muted-foreground"
              title={run.request}
            >
              {run.request}
            </p>
          )}
        </div>
        <RunStatus current={run.current} />
        <div className="flex-1" />
        <DeleteRunButton
          name={run.name}
          onDelete={async () => {
            await deleteRun(root, run.runId)
            await navigate({ to: "/runs" })
          }}
        />
      </header>
      <div className="relative flex min-h-0 flex-1">
        {artifact ? (
          <ArtifactViewer
            key={artifact.path}
            root={root}
            runId={run.runId}
            artifact={artifact}
            onClose={() => setArtifact(undefined)}
          />
        ) : (
          <div className="flex min-w-0 flex-1 flex-col">
            {missing.length > 0 && (
              <div
                role="alert"
                className="flex items-start gap-2 border-b bg-warning-muted px-4 py-2 text-sm text-warning"
              >
                <TriangleAlertIcon
                  className="mt-0.5 size-4 shrink-0"
                  aria-hidden="true"
                />
                <span>
                  The workflow changed since this run started. Not in the graph:{" "}
                  <span className="font-mono">{missing.join(", ")}</span>
                </span>
              </div>
            )}
            {workflow ? (
              <>
                <FlowToolbar />
                <FlowPane
                  readOnly
                  steps={workflow.steps}
                  start={workflow.start}
                  status={status}
                  selected={selected}
                  onSelect={select}
                />
              </>
            ) : workflow === null ? (
              <div className="min-h-0 flex-1 overflow-auto p-4">
                <p role="alert" className="mb-3 text-sm text-warning">
                  The workflow “{run.workflowName}” could not be loaded, so the
                  graph is unavailable.
                </p>
                <ul className="flex flex-col gap-1.5">
                  {[...executions].map(([id, list]) => (
                    <li key={id}>
                      <button
                        type="button"
                        aria-pressed={selected === id}
                        onClick={() => select(id)}
                        className="flex h-9 w-full max-w-md cursor-pointer items-center gap-2 rounded-md border px-3 text-left text-sm hover:bg-accent aria-pressed:border-primary aria-pressed:bg-selection"
                      >
                        <span className="flex-1 truncate font-medium">
                          {humanize(id)}
                        </span>
                        {list.some((e) => e.active) && (
                          <Loader2Icon
                            className="size-3.5 animate-spin text-primary"
                            aria-label="Running"
                          />
                        )}
                        <span className="text-xs text-muted-foreground">
                          {list.length}{" "}
                          {list.length === 1 ? "execution" : "executions"}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            ) : (
              <p role="status" className="p-4">
                Loading workflow…
              </p>
            )}
          </div>
        )}
        {selected && (
          <aside
            aria-label={`Step ${humanize(selected)}`}
            className="absolute inset-y-0 right-0 z-10 flex w-[380px] max-w-full flex-col border-l bg-background shadow-lg min-[1280px]:relative min-[1280px]:shrink-0 min-[1280px]:shadow-none"
          >
            <ExecutionPanel
              nodeId={selected}
              kind={kind}
              executions={executions.get(selected) ?? []}
              open={artifact?.path}
              onOpen={setArtifact}
              existingArtifactPaths={run.existingArtifactPaths}
              onClose={() => {
                setSelected(undefined)
                setArtifact(undefined)
              }}
            />
          </aside>
        )}
      </div>
    </div>
  )
}
