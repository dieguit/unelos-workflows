import { useState } from "react"
import { useRouteContext } from "@tanstack/react-router"
import {
  ChevronsUpDownIcon,
  CopyIcon,
  FolderGit2Icon,
  FolderOpenIcon,
  FolderSearchIcon,
} from "lucide-react"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { useProjectActions } from "@/hooks/use-project-actions"
import { shortPath } from "@/lib/projects"
import { cn } from "@/lib/utils"

export function ProjectSelector() {
  const { projectState, project } = useRouteContext({ from: "__root__" })
  const { openPath, pickAndOpen, copyPath, reveal } = useProjectActions()
  const [open, setOpen] = useState(false)
  const { home } = projectState
  const others = projectState.recents.filter((r) => r.path !== project?.path)

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <Tooltip>
        <TooltipTrigger
          render={
            <DropdownMenuTrigger
              onContextMenu={(e) => {
                e.preventDefault()
                setOpen(true)
              }}
              className="flex w-full items-center gap-2.5 rounded-md border bg-card p-2 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
          }
        >
          <span className="flex size-7 shrink-0 items-center justify-center rounded-sm bg-secondary">
            <FolderGit2Icon
              aria-hidden="true"
              className={cn(
                "size-4",
                project ? "text-primary" : "text-muted-foreground"
              )}
            />
          </span>
          <span className="min-w-0 flex-1">
            <span className="block truncate text-base font-semibold">
              {project ? project.name : "No project open"}
            </span>
            <span
              className={cn(
                "block truncate text-xs text-muted-foreground",
                project && "font-mono"
              )}
            >
              {project
                ? shortPath(project.path, home)
                : "Open a folder to begin"}
            </span>
          </span>
          <ChevronsUpDownIcon
            aria-hidden="true"
            className="size-3.5 shrink-0 text-muted-foreground"
          />
        </TooltipTrigger>
        {project && (
          <TooltipContent side="right" className="flex-col items-start">
            <span className="font-mono">{project.path}</span>
            <span className="opacity-70">
              Right-click to copy path or reveal in file manager
            </span>
          </TooltipContent>
        )}
      </Tooltip>
      <DropdownMenuContent align="start" className="w-64">
        {others.map((r) => (
          <DropdownMenuItem
            key={r.path}
            onClick={() => void openPath(r.path)}
            className={cn(!r.available && "text-foreground-disabled")}
          >
            <span className="min-w-0 flex-1 truncate">{r.name}</span>
            {!r.available && (
              <span className="text-xs text-warning">Unavailable</span>
            )}
          </DropdownMenuItem>
        ))}
        {others.length > 0 && <DropdownMenuSeparator />}
        <DropdownMenuItem onClick={() => void pickAndOpen()}>
          <FolderOpenIcon /> Open folder…
          <DropdownMenuShortcut>⌘O</DropdownMenuShortcut>
        </DropdownMenuItem>
        {project && (
          <>
            <DropdownMenuItem onClick={() => void copyPath(project.path)}>
              <CopyIcon /> Copy path
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => void reveal(project.path)}>
              <FolderSearchIcon /> Reveal in file manager
            </DropdownMenuItem>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
