import { CircleAlertIcon, FolderOpenIcon, FolderTreeIcon, LoaderCircleIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { isRemoteSource, repoLabel, splitError } from "../-lib/import-plan"

export function ImportSourceStep({
  source,
  onChange,
  onSubmit,
  onBrowse,
  loading,
  error,
}: {
  source: string
  onChange: (value: string) => void
  onSubmit: () => void
  onBrowse: () => void
  loading: boolean
  error: string | null
}) {
  const failure = error ? splitError(error) : null
  const repo = repoLabel(source)
  return (
    <form
      id="import-source-form"
      className="flex flex-col gap-3"
      onSubmit={(e) => {
        e.preventDefault()
        onSubmit()
      }}
    >
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="import-source">Folder path or git URL</Label>
        <div className="flex gap-2">
          <Input
            id="import-source"
            autoFocus
            spellCheck={false}
            autoComplete="off"
            className="h-9 flex-1 font-mono"
            placeholder="~/path/to/folder or a GitHub folder link"
            value={source}
            disabled={loading}
            aria-invalid={failure ? true : undefined}
            aria-describedby="import-source-note"
            onChange={(e) => onChange(e.target.value)}
          />
          <Button type="button" variant="outline" size="lg" disabled={loading} onClick={onBrowse}>
            <FolderOpenIcon /> Browse…
          </Button>
        </div>
        <div id="import-source-note" aria-live="polite" className="text-sm">
          {loading ? (
            <p role="status" className="flex items-start gap-2 text-foreground-dim">
              <LoaderCircleIcon aria-hidden className="mt-0.5 size-4 shrink-0 animate-spin" />
              {isRemoteSource(source) ? (
                <span className="flex flex-col">
                  <span>Fetching and checking…</span>
                  <span className="text-muted-foreground">
                    Cloning {repo ?? "repository"}. This can take a few seconds.
                  </span>
                </span>
              ) : (
                "Checking folder…"
              )}
            </p>
          ) : failure ? (
            <p role="alert" className="flex items-start gap-2 text-destructive">
              <CircleAlertIcon aria-hidden className="mt-0.5 size-4 shrink-0" />
              <span className="flex flex-col gap-0.5">
                <span>{failure.title}</span>
                {failure.hint ? <span className="text-xs text-foreground-dim">{failure.hint}</span> : null}
              </span>
            </p>
          ) : (
            <p className="text-foreground-dim">
              For a repository, add a branch and folder:{" "}
              <span className="font-mono text-xs">https://github.com/org/repo#main:workflows/research</span>
            </p>
          )}
        </div>
      </div>
      <div className="flex gap-3 rounded-lg border bg-background/40 p-3">
        <FolderTreeIcon aria-hidden className="mt-0.5 size-4 shrink-0 text-foreground-dim" />
        <div className="flex flex-col gap-0.5 text-sm">
          <p className="font-medium">One workflow per import</p>
          <p className="text-foreground-dim">
            The folder should contain a single workflow plus the agents, tasks, skills and MCP servers it uses. Nothing is
            written to your project until you confirm.
          </p>
        </div>
      </div>
    </form>
  )
}
