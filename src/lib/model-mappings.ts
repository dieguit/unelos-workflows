import type { AgentSummary } from "./projects"
import { trackedInvoke } from "./tauri"

export type MappedModel = { model: string; effort?: string }
export type ModelMappings = Record<string, Record<string, MappedModel>>
export type ModelInfo = { id: string; efforts: string[] }
export const listModelMappings = () => trackedInvoke<ModelMappings>("list_model_mappings")
export const saveModelMapping = (original: string | null, alias: string, models: Record<string, MappedModel>) =>
  trackedInvoke<ModelMappings>("save_model_mapping", { original, alias, models })
export const deleteModelMapping = (alias: string) => trackedInvoke<ModelMappings>("delete_model_mapping", { alias })
export const listHarnessModels = (id: string) => trackedInvoke<ModelInfo[]>("harness_models", { id })

/** Agent names using `alias`; null when no project is open. */
export const usedBy = (agents: AgentSummary[] | null, alias: string) =>
  agents?.filter((a) => a.model === alias).map((a) => a.name) ?? null

export const usedByLabel = (names: string[] | null) =>
  !names?.length ? "—" : `${names.length} agent${names.length === 1 ? "" : "s"}`
