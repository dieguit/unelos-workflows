import type { AgentModelOptions } from "@/lib/projects"

export function directEfforts(options: AgentModelOptions, harness: string, model: string): string[] | undefined {
  return options.harnesses.find((h) => h.id === harness && h.error === null)
    ?.models.find((m) => m.id === model)?.efforts
}

export function aliasEfforts(options: AgentModelOptions, alias: string): string[] | undefined {
  const lists = Object.entries(options.mappings[alias] ?? {})
    .map(([harness, mapped]) => directEfforts(options, harness, mapped.model))
    .filter((efforts) => efforts !== undefined)
  return lists[0]?.filter((effort) => lists.every((list) => list.includes(effort)))
}

export function inheritedLabel(options: AgentModelOptions, alias: string): string {
  const summary = Object.entries(options.mappings[alias] ?? {}).map(([id, mapped]) =>
    `${options.harnesses.find((h) => h.id === id)?.name ?? id}: ${mapped.effort ?? "default"}`
  ).join(" · ")
  return `Mapping default (${summary || "none"})`
}
