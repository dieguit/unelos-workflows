import { useForm } from "@tanstack/react-form"
import { skillSchema } from "@/lib/skill-schema"

export type SkillDraft = { name: string; description: string; body: string }
export function useSkillForm(
  defaultValues: SkillDraft,
  save: (
    value: SkillDraft,
    closeEditor: boolean
  ) => Promise<SkillDraft | undefined>
) {
  return useForm({
    defaultValues,
    onSubmitMeta: { body: null as string | null, closeEditor: false },
    validators: {
      onChange: skillSchema,
      onBlur: skillSchema,
      onSubmit: skillSchema,
    },
    onSubmit: async ({ value, meta, formApi }) => {
      const saved = await save(
        { ...value, body: meta.body ?? value.body },
        meta.closeEditor
      )
      if (saved) formApi.reset(saved)
    },
  })
}
export type SkillForm = ReturnType<typeof useSkillForm>
