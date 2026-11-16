import type { ModelMappings } from "@/lib/model-mappings"
import { usedBy } from "@/lib/model-mappings"
import type { AgentSummary } from "@/lib/projects"

export type MissingAlias = { alias: string; agents: string[] }

/** Aliases with no model for `harnessId`; ones used by project agents first, then alphabetical. */
export function missingAliases(
  mappings: ModelMappings,
  harnessId: string,
  agents: AgentSummary[] | null
): MissingAlias[] {
  return Object.keys(mappings)
    .filter((alias) => !mappings[alias][harnessId])
    .map((alias) => ({ alias, agents: usedBy(agents, alias) ?? [] }))
    .sort(
      (a, b) =>
        Number(!!b.agents.length) - Number(!!a.agents.length) ||
        a.alias.localeCompare(b.alias)
    )
}
