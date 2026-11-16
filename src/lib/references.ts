import { getProjectReferences, type ProjectReferences, type Reference } from "./projects"

/** Who references an item; `of` is null when references are unavailable (no project, failed scan). */
export type UsedBy = { of: (id: string) => Reference[] | null; project?: string }

/** A failed scan or no open project shouldn't hide the page. */
export const loadReferences = (root?: string) =>
  root ? getProjectReferences(root).catch(() => null) : Promise.resolve(null)

/** Global items are referenced as `global/<id>`; the count is for the open project, named on global pages. */
export const usedByFor = (
  map: ProjectReferences[keyof ProjectReferences] | undefined | null,
  scope: { global?: boolean; project?: string } = {}
): UsedBy => ({
  of: (id) => (map ? (map[scope.global ? `global/${id}` : id] ?? []) : null),
  project: scope.global ? scope.project : undefined,
})

const KINDS: Reference["kind"][] = ["workflow", "agent", "task"]

export const groupReferences = (refs: Reference[]) =>
  KINDS.map((kind) => ({ kind, items: refs.filter((r) => r.kind === kind) })).filter((g) => g.items.length)

export const usedByLabel = (refs: Reference[] | null) =>
  !refs?.length
    ? "—"
    : groupReferences(refs)
        .map((g) => `${g.items.length} ${g.kind}${g.items.length === 1 ? "" : "s"}`)
        .join(", ")
