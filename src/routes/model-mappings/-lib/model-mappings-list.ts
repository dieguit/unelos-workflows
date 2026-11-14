import type { ModelMappings } from "@/lib/model-mappings"
import type { AgentSummary } from "@/lib/projects"

export const ALIAS_RE = /^[a-z0-9]+([-_.][a-z0-9]+)*$/

export const aliasError = (alias: string) =>
  !alias
    ? "Enter an alias."
    : ALIAS_RE.test(alias)
      ? undefined
      : "Use lowercase letters and numbers, joined by “-”, “_” or “.”"

/** Agent names using `alias`; null when no project is open. */
export const usedBy = (agents: AgentSummary[] | null, alias: string) =>
  agents?.filter((a) => a.model === alias).map((a) => a.name) ?? null

export const usedByLabel = (names: string[] | null) =>
  !names?.length ? "—" : `${names.length} agent${names.length === 1 ? "" : "s"}`

export function filterMappings(mappings: ModelMappings, query: string, harnesses: { id: string }[]) {
  const needle = query.trim().toLowerCase()
  return Object.keys(mappings)
    .sort()
    .filter((alias) =>
      [alias, ...harnesses.map((h) => mappings[alias][h.id] ?? "")].some(
        (text) => text.toLowerCase().includes(needle)
      )
    )
}
