import { Link } from "@tanstack/react-router"
import type { Reference } from "@/lib/projects"
import { groupReferences, usedByLabel } from "@/lib/references"
import { ListCell, ListHead } from "@/components/list-table"

export function UsedByHead({ project, className }: { project?: string; className?: string }) {
  return (
    <ListHead className={className} title={project ? `Used in ${project}` : undefined}>
      Used by
    </ListHead>
  )
}

export function UsedByCell({ refs }: { refs: Reference[] | null }) {
  return <ListCell className="text-foreground-dim">{usedByLabel(refs)}</ListCell>
}

function Name({ r }: { r: Reference }) {
  // Only tasks have a route that opens an item from a link.
  return r.kind === "task" ? (
    <Link to="/tasks" search={{ task: r.id }} className="hover:underline">
      {r.name}
    </Link>
  ) : (
    r.name
  )
}

/** Bottom "Used by" row of an inspector. */
export function UsedByRow({ refs, project }: { refs: Reference[] | null; project?: string }) {
  const groups = refs ? groupReferences(refs) : []
  return (
    <div className="grid grid-cols-[76px_minmax(0,1fr)] gap-x-2 text-sm">
      <span>Used by</span>
      <div className="flex flex-col gap-1 [overflow-wrap:anywhere] text-foreground-dim">
        {groups.length ? (
          groups.map((g) => (
            <p key={g.kind}>
              <span className="text-xs text-muted-foreground">{g.kind}s </span>
              {g.items.map((r, i) => (
                <span key={r.id}>
                  {i > 0 ? ", " : null}
                  <Name r={r} />
                </span>
              ))}
            </p>
          ))
        ) : (
          <span>—</span>
        )}
        {project && refs ? <span className="text-xs text-muted-foreground">in {project}</span> : null}
      </div>
    </div>
  )
}
