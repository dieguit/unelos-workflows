import type { ModelMappings } from "@/lib/model-mappings"

export const ALIAS_RE = /^[a-z0-9]+([-_.][a-z0-9]+)*$/

export const aliasError = (alias: string) =>
  !alias
    ? "Enter an alias."
    : ALIAS_RE.test(alias)
      ? undefined
      : "Use lowercase letters and numbers, joined by “-”, “_” or “.”"

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
