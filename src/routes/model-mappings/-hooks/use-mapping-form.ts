import { useForm } from "@tanstack/react-form"
import { mappingSchema, type MappingDraft } from "../-lib/mapping-schema"

export function useMappingForm(
  defaultValues: MappingDraft,
  save: (value: MappingDraft) => Promise<MappingDraft | undefined>
) {
  return useForm({
    defaultValues,
    validators: {
      onChange: mappingSchema,
      onBlur: mappingSchema,
      onSubmit: mappingSchema,
    },
    onSubmit: async ({ value, formApi }) => {
      const saved = await save(value)
      if (saved) formApi.reset(saved)
    },
  })
}
export type MappingForm = ReturnType<typeof useMappingForm>
