import { ArrowRightIcon, CheckIcon, GitBranchIcon, TerminalIcon, TriangleAlertIcon, WorkflowIcon } from "lucide-react"
import { Fragment } from "react"
import { kindIcon } from "./import-kind-badge"
import { humanize, isRemoteSource, type ImportKind, type ImportPlan } from "../-lib/import-plan"

const groups: [ImportKind, string][] = [
  ["agent", "Agents"],
  ["task", "Tasks"],
  ["skill", "Skills"],
  ["mcp", "MCP servers"],
]

function Message({ text }: { text: string }) {
  return text.split("`").map((part, i) =>
    i % 2 ? (
      <code key={i} className="font-mono text-xs">
        {part}
      </code>
    ) : (
      <Fragment key={i}>{part}</Fragment>
    )
  )
}

export function ImportReviewStep({
  plan,
  trust,
  onTrust,
}: {
  plan: ImportPlan
  trust: boolean
  onTrust: (value: boolean) => void
}) {
  const reused = new Set(plan.reused)
  return (
    <div className="flex flex-col gap-5">
      <div className="flex gap-3">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-primary-muted text-primary">
          <WorkflowIcon aria-hidden className="size-5" />
        </span>
        <div className="flex min-w-0 flex-col gap-1">
          <h3 className="flex flex-wrap items-baseline gap-x-2">
            <span className="text-lg font-semibold">{humanize(plan.workflow.id)}</span>
            <span className="font-mono text-xs text-muted-foreground">{plan.workflow.id}</span>
          </h3>
          {plan.workflow.description ? <p className="text-sm text-foreground-dim">{plan.workflow.description}</p> : null}
          <p className="flex items-center gap-1.5 font-mono text-xs text-muted-foreground">
            {isRemoteSource(plan.source) ? <GitBranchIcon aria-hidden className="size-3 shrink-0" /> : null}
            <span className="break-all">{plan.source}</span>
          </p>
        </div>
      </div>

      <section aria-labelledby="import-included" className="flex flex-col gap-2">
        <h4 id="import-included" className="text-xs font-medium tracking-[0.6px] text-muted-foreground uppercase">
          What’s included
        </h4>
        <ul className="divide-y rounded-lg border bg-card">
          {groups.map(([kind, label]) => {
            const ids = plan.items.filter((i) => i.kind === kind).map((i) => i.id)
            if (!ids.length) return null
            const Icon = kindIcon[kind]
            return (
              <li key={kind} className="flex items-start gap-3 px-3 py-2 text-sm">
                <span className="flex w-32 shrink-0 items-center gap-2">
                  <Icon aria-hidden className="size-3.5 text-foreground-dim" />
                  {label}
                  <span className="text-foreground-dim">{ids.length}</span>
                </span>
                <span className="min-w-0 flex-1 font-mono text-xs leading-5 text-foreground-dim">
                  {ids.map((id, i) => (
                    <Fragment key={id}>
                      {i ? ", " : ""}
                      <span title={reused.has(`${kind}:${id}`) ? "Already in this project, reused as is" : undefined}>{id}</span>
                    </Fragment>
                  ))}
                </span>
              </li>
            )
          })}
        </ul>
      </section>

      {plan.warnings.length ? (
        <section aria-labelledby="import-warnings" className="flex flex-col gap-1.5 rounded-lg border border-warning/30 bg-warning-muted p-3 text-sm">
          <h4 id="import-warnings" className="flex items-center gap-2 font-medium text-warning">
            <TriangleAlertIcon aria-hidden className="size-4" />
            {plan.warnings.length} {plan.warnings.length === 1 ? "warning" : "warnings"}
          </h4>
          <ul className="flex flex-col gap-1 pl-1">
            {plan.warnings.map((w, i) => (
              <li key={i} className="flex gap-2">
                <span aria-hidden className="mt-2 size-1 shrink-0 rounded-full bg-warning" />
                <span>
                  <Message text={w.message} />
                </span>
              </li>
            ))}
          </ul>
          <p className="text-xs text-foreground-dim">You can fix these after importing.</p>
        </section>
      ) : null}

      {plan.mcps.length ? (
        <section aria-labelledby="import-mcps" className="flex flex-col gap-3">
          <div className="flex flex-col gap-1">
            <h4 id="import-mcps" className="text-xs font-medium tracking-[0.6px] text-muted-foreground uppercase">
              MCP servers that will run on this machine
            </h4>
            <p className="text-sm text-foreground-dim">
              Check each command before trusting it. These run with your user’s permissions whenever the workflow uses them.
            </p>
          </div>
          <ul className="divide-y rounded-lg border bg-card">
            {plan.mcps.map((m) => (
              <li key={m.id} className="flex flex-col gap-1.5 p-3">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-sm font-medium">{m.id}</span>
                  <span className="rounded border px-1.5 text-[11px] text-foreground-dim">{m.command ? "command" : "url"}</span>
                </div>
                <p className="flex items-start gap-2 rounded-md bg-background px-2 py-1.5 font-mono text-xs">
                  {m.command ? (
                    <TerminalIcon aria-label="command" className="mt-0.5 size-3 shrink-0 text-muted-foreground" />
                  ) : (
                    <ArrowRightIcon aria-label="url" className="mt-0.5 size-3 shrink-0 text-muted-foreground" />
                  )}
                  <span className="min-w-0 break-all whitespace-pre-wrap">{m.command ?? m.url}</span>
                </p>
              </li>
            ))}
          </ul>
          <label className="flex w-fit cursor-pointer items-center gap-2 text-sm">
            <span className="relative flex size-4">
              <input
                type="checkbox"
                autoFocus
                className="peer size-4 cursor-pointer appearance-none rounded-[4px] border border-border-strong bg-transparent outline-none checked:border-primary checked:bg-primary focus-visible:ring-3 focus-visible:ring-ring/50"
                checked={trust}
                onChange={(e) => onTrust(e.target.checked)}
              />
              <CheckIcon aria-hidden className="pointer-events-none absolute inset-0 m-auto size-3 text-primary-foreground opacity-0 peer-checked:opacity-100" />
            </span>
            I trust these MCP servers to run on my machine
          </label>
        </section>
      ) : null}
    </div>
  )
}
