import { trackedInvoke } from "./tauri"

export type ModelMappings = Record<string, Record<string, string>>
export const listModelMappings = () => trackedInvoke<ModelMappings>("list_model_mappings")
export const saveModelMapping = (original: string | null, alias: string, models: Record<string, string>) =>
  trackedInvoke<ModelMappings>("save_model_mapping", { original, alias, models })
export const deleteModelMapping = (alias: string) => trackedInvoke<ModelMappings>("delete_model_mapping", { alias })
export const listHarnessModels = (id: string) => trackedInvoke<string[]>("harness_models", { id })
