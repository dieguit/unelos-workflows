import { useState } from "react"
import { TriangleAlertIcon } from "lucide-react"
import { BusyContent } from "@/components/busy-content"
import { ModelPicker } from "@/components/model-picker"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog"
import { useHarnessModels } from "@/hooks/use-harness-models"
import type { Harness } from "@/lib/harnesses"
import { listModelMappings, saveModelMapping } from "@/lib/model-mappings"
import type { MissingAlias } from "../-lib/missing-mappings"

export function MapModelsDialog({
  harness,
  rows,
  onClose,
}: {
  harness: Harness
  rows: MissingAlias[]
  onClose: () => void
}) {
  const [values, setValues] = useState<Record<string, string>>({})
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  const { models, load } = useHarnessModels()
  const used = rows.filter((r) => r.agents.length)
  const filled = Object.keys(values).length

  const save = async () => {
    setBusy(true)
    setError(undefined)
    try {
      // Re-read so models other harnesses got meanwhile are preserved.
      const current = await listModelMappings()
      for (const [alias, model] of Object.entries(values))
        await saveModelMapping(alias, alias, { ...current[alias], [harness.id]: model })
      onClose()
    } catch (e) {
      setError(String(e))
      setBusy(false)
    }
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent inert={busy} className="sm:max-w-lg">
        <DialogTitle>Map models for {harness.name}</DialogTitle>
        <DialogDescription>
          These aliases have no {harness.name} model yet. Leave a row empty to
          keep it unmapped.
          {used.length
            ? ` Agents using an unmapped alias won’t get ${harness.name} files generated.`
            : ""}
        </DialogDescription>
        <form
          id="map-models-form"
          className="flex max-h-[min(24rem,60vh)] flex-col gap-3 overflow-y-auto"
          onSubmit={(e) => {
            e.preventDefault()
            if (filled && !busy) void save()
          }}
        >
          {rows.map(({ alias, agents }, i) => (
            <div
              key={alias}
              className="grid grid-cols-[minmax(0,8rem)_minmax(0,1fr)] items-start gap-x-3"
            >
              <label htmlFor={`map-${alias}`} className="flex flex-col gap-0.5 pt-1.5">
                <span className="truncate font-mono text-sm">{alias}</span>
                {agents.length ? (
                  <span className="flex items-center gap-1 text-xs text-warning">
                    <TriangleAlertIcon aria-hidden="true" className="size-3 shrink-0" />
                    {agents.length} agent{agents.length === 1 ? "" : "s"}
                  </span>
                ) : (
                  <span className="text-xs text-muted-foreground">Not used here</span>
                )}
              </label>
              <ModelPicker
                id={`map-${alias}`}
                label={`${alias} ${harness.name}`}
                value={values[alias]}
                neutral={!agents.length}
                models={models[harness.id]}
                onOpen={() => load(harness.id)}
                onChange={(model) => setValues((v) => ({ ...v, [alias]: model }))}
              />
              {i === used.length - 1 && used.length < rows.length ? (
                <div className="col-span-2 mt-3 border-t" aria-hidden="true" />
              ) : null}
            </div>
          ))}
        </form>
        {error ? (
          <p role="alert" className="text-sm [overflow-wrap:anywhere] text-destructive">
            {error}
          </p>
        ) : null}
        <DialogFooter>
          <Button variant="outline" disabled={busy} onClick={onClose}>
            Skip
          </Button>
          <Button type="submit" form="map-models-form" disabled={!filled || busy}>
            <BusyContent busy={busy}>Save mappings</BusyContent>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
