import { ArrowRightIcon, CheckIcon, CircleAlertIcon } from "lucide-react"
import { useState } from "react"
import { Input } from "@/components/ui/input"
import { ImportKindBadge } from "./import-kind-badge"
import { causeText, type ImportPlan, type ImportRenames } from "../-lib/import-plan"

export function ImportNamesStep({
  plan,
  renames,
  notice,
  onRename,
}: {
  plan: ImportPlan
  renames: ImportRenames
  notice: string | null
  onRename: (key: string, value: string) => void
}) {
  const [focusKey] = useState(() => (plan.conflicts.find((c) => c.error) ?? plan.conflicts[0])?.key)
  const count = plan.conflicts.length
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      {notice ? (
        <p role="alert" className="flex items-start gap-2 rounded-lg border border-warning/30 bg-warning-muted p-3 text-sm">
          <CircleAlertIcon aria-hidden className="mt-0.5 size-4 shrink-0 text-warning" />
          {notice}
        </p>
      ) : null}
      <div className="flex flex-col gap-1">
        <h3 className="text-base font-semibold">
          {count} {count === 1 ? "item needs" : "items need"} a new name
        </h3>
        <p className="text-sm text-foreground-dim">
          They share a name with something already in this project but have different content. They’ll be imported under the
          names below, and references inside this workflow are updated to match.
        </p>
      </div>
      <div role="table" aria-label="Names to import as" className="flex min-h-0 flex-col overflow-hidden rounded-lg border bg-card">
        <div role="row" className="grid grid-cols-[5.5rem_1fr_1rem_minmax(0,1fr)] gap-x-3 border-b px-4 py-2 text-xs font-medium tracking-[0.6px] text-muted-foreground uppercase">
          <span role="columnheader">Kind</span>
          <span role="columnheader">Name in source</span>
          <span aria-hidden />
          <span role="columnheader">Import as</span>
        </div>
        <ul role="rowgroup" className="min-h-0 overflow-y-auto">
          {plan.conflicts.map((c) => {
            const inputId = `import-name-${c.key}`
            const errorId = `${inputId}-error`
            const valid = !c.error
            return (
              <li key={c.key} role="row" className="grid grid-cols-[5.5rem_1fr_1rem_minmax(0,1fr)] items-start gap-x-3 border-b px-4 py-3 last:border-b-0">
                <span role="cell" className="pt-0.5">
                  <ImportKindBadge kind={c.kind} />
                </span>
                <span role="cell" className="flex min-w-0 flex-col gap-1">
                  <label htmlFor={inputId} className="truncate pt-0.5 font-mono text-sm">
                    {c.id}
                  </label>
                  {c.reason === "referencesRenamed" && c.causedBy ? (
                    <span className="text-xs text-muted-foreground">{causeText(c.causedBy)}</span>
                  ) : null}
                </span>
                <ArrowRightIcon aria-hidden className="mt-1.5 size-3.5 text-muted-foreground" />
                <span role="cell" className="flex min-w-0 flex-col gap-1">
                  <span className="relative">
                    <Input
                      id={inputId}
                      autoFocus={c.key === focusKey}
                      spellCheck={false}
                      autoComplete="off"
                      className="pr-7 font-mono"
                      value={renames[c.key] ?? c.suggested}
                      aria-invalid={c.error ? true : undefined}
                      aria-describedby={c.error ? errorId : undefined}
                      onChange={(e) => onRename(c.key, e.target.value)}
                    />
                    {valid ? <CheckIcon aria-label="Name is available" className="absolute top-2 right-2 size-4 text-success" /> : null}
                  </span>
                  {c.error ? (
                    <span id={errorId} className="flex items-start gap-1.5 text-xs text-destructive">
                      <CircleAlertIcon aria-hidden className="mt-0.5 size-3.5 shrink-0" />
                      {c.error}
                    </span>
                  ) : null}
                </span>
              </li>
            )
          })}
        </ul>
      </div>
    </div>
  )
}
