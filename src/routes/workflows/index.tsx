import { useEffect, useRef, useState } from "react"
import { createFileRoute, useRouter } from "@tanstack/react-router"
import { SearchIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { listWorkflows, requireProject } from "@/lib/projects"
import { countLabel, filterWorkflows } from "./-lib/filter"
import {
  NewWorkflowButton,
  WorkflowsTable,
} from "./-components/workflows-table"
import { NoResults, NoWorkflows } from "./-components/workflows-empty"

export const Route = createFileRoute("/workflows/")({
  beforeLoad: requireProject,
  loader: ({ context }) => listWorkflows(context.project.path),
  component: WorkflowsHome,
  errorComponent: WorkflowsError,
})

function Header() {
  return (
    <header className="flex items-end justify-between gap-4">
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold">Workflows</h1>
        <p className="text-base text-foreground-dim">
          Reusable process definitions for this project. Open one to edit its
          steps, routes and inputs.
        </p>
      </div>
      <NewWorkflowButton />
    </header>
  )
}

const isEditable = (el: EventTarget | null) =>
  el instanceof HTMLElement &&
  (el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName))

function WorkflowsHome() {
  const workflows = Route.useLoaderData()
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)
  const shown = filterWorkflows(workflows, query)
  const filtering = query.trim() !== ""

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "/" && !isEditable(e.target)) {
        e.preventDefault()
        search.current?.focus()
      }
    }
    document.addEventListener("keydown", onKey)
    return () => document.removeEventListener("keydown", onKey)
  }, [])

  if (workflows.length === 0)
    return (
      <div className="flex flex-col gap-6">
        <Header />
        <NoWorkflows />
      </div>
    )

  return (
    <div className="flex flex-col gap-4">
      <Header />
      <div className="mt-1 flex items-center gap-3">
        <div className="relative w-70">
          <SearchIcon
            aria-hidden="true"
            className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            ref={search}
            type="search"
            aria-label="Search workflows"
            placeholder="Search workflows"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== "Escape") return
              if (query) setQuery("")
              else e.currentTarget.blur()
            }}
            className="rounded-md bg-background pr-8 pl-8 md:text-base [&::-webkit-search-cancel-button]:hidden"
          />
          <kbd
            aria-hidden="true"
            className="absolute top-1/2 right-2 -translate-y-1/2 rounded-sm border border-border-strong px-1.25 py-px font-mono text-xs text-muted-foreground"
          >
            /
          </kbd>
        </div>
        <span
          data-workflow-count
          aria-live="polite"
          className="text-sm text-muted-foreground"
        >
          {countLabel(shown.length, workflows.length, filtering)}
        </span>
      </div>
      {shown.length === 0 ? (
        <NoResults query={query.trim()} onClear={() => setQuery("")} />
      ) : (
        <WorkflowsTable workflows={shown} />
      )}
    </div>
  )
}

function WorkflowsError({ error }: { error: unknown }) {
  const router = useRouter()
  return (
    <div className="flex flex-col gap-6">
      <Header />
      <div className="flex flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
        <p className="text-sm">
          Couldn’t load workflows:{" "}
          {error instanceof Error ? error.message : String(error)}
        </p>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void router.invalidate()}
        >
          Retry
        </Button>
      </div>
    </div>
  )
}
