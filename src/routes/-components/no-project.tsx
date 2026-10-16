import { useRouteContext } from "@tanstack/react-router"
import { FolderOpenIcon, TriangleAlertIcon, FolderIcon } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { useProjectActions } from "@/hooks/use-project-actions"
import { relativeTime, shortPath } from "@/lib/projects"
import { cn } from "@/lib/utils"

export function NoProject() {
  const { projectState } = useRouteContext({ from: "__root__" })
  const { pickAndOpen, openPath, removeRecent } = useProjectActions()
  const { home, recents } = projectState
  const lost = recents[0] && !recents[0].available ? recents[0] : null

  return (
    <div className="mx-auto flex max-w-xl flex-col gap-8 pt-24">
      {lost && (
        <Alert className="border-warning/40 bg-warning-muted">
          <TriangleAlertIcon className="text-warning" />
          <AlertTitle>{lost.name} couldn’t be opened</AlertTitle>
          <AlertDescription>
            The project at <span className="break-all">{lost.path}</span> is
            unavailable. The folder may have moved or its .unelos/project.yaml
            may be missing.
            <div className="mt-3 flex gap-2">
              <Button size="sm" variant="outline" onClick={() => void pickAndOpen()}>
                <FolderOpenIcon /> Locate folder…
              </Button>
              <Button size="sm" variant="ghost" onClick={() => void removeRecent(lost.path)}>
                Remove from recents
              </Button>
            </div>
          </AlertDescription>
        </Alert>
      )}

      <section className="flex flex-col items-start gap-3">
        <span className="flex size-9 items-center justify-center rounded-md bg-secondary">
          <FolderIcon className="size-5 text-primary" aria-hidden="true" />
        </span>
        <h1 className="text-xl font-semibold">Open a project to get started</h1>
        <p className="text-sm text-foreground-dim">
          Unelos reads workflows, task definitions and agents from a project folder.
        </p>
        <div className="flex items-center gap-2">
          <Button autoFocus onClick={() => void pickAndOpen()}>
            <FolderOpenIcon /> Open project folder
          </Button>
          <kbd className="font-mono text-xs text-muted-foreground">⌘O</kbd>
        </div>
      </section>

      {recents.length > 0 && (
        <section aria-labelledby="recent-heading" className="flex flex-col gap-1">
          <h2 id="recent-heading" className="px-2 text-xs font-medium tracking-wide text-muted-foreground">
            RECENT PROJECTS
          </h2>
          <ul className="flex flex-col">
            {recents.map((r) => (
              <li key={r.path}>
                <button
                  type="button"
                  onClick={() => void openPath(r.path)}
                  className="flex h-9 w-full items-center gap-3 rounded-md px-2 text-left outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <FolderIcon aria-hidden="true" className={cn("size-4 shrink-0", r.available ? "text-primary" : "text-foreground-disabled")} />
                  <span className={cn("w-40 truncate text-base font-medium", !r.available && "text-foreground-disabled")}>{r.name}</span>
                  <Tooltip>
                    <TooltipTrigger render={<span className={cn("min-w-0 flex-1 truncate font-mono text-xs", r.available ? "text-muted-foreground" : "text-foreground-disabled")} />}>
                      {shortPath(r.path, home, 2)}
                    </TooltipTrigger>
                    <TooltipContent>{r.path}</TooltipContent>
                  </Tooltip>
                  <span className={cn("shrink-0 text-xs", r.available ? "text-muted-foreground" : "text-warning")}>
                    {r.available ? relativeTime(r.openedAt) : "Unavailable"}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  )
}
