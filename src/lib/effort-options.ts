import type { ModelInfo } from "./model-mappings"

export function effortChoices(efforts: string[] | undefined, saved: string | undefined) {
  const choices = (efforts ?? []).map((value) => ({ value, stale: false }))
  if (saved !== undefined && !efforts?.includes(saved))
    choices.push({ value: saved, stale: efforts !== undefined })
  return choices
}

export function effortsFor(models: ModelInfo[] | undefined, model: string | undefined) {
  return models?.find((entry) => entry.id === model)?.efforts
}
