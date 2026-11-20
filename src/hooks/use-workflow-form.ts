import { useForm } from "@tanstack/react-form"
import type { WorkflowDraft } from "@/lib/projects"
import { workflowSettingsSchema } from "@/lib/workflow-settings-schema"

export const toDraft = (doc: WorkflowDraft): WorkflowDraft => ({
  name: doc.name,
  description: doc.description,
  start: doc.start,
  inputs: doc.inputs,
  steps: doc.steps,
})

export function useWorkflowForm(
  saved: WorkflowDraft,
  submit: (value: WorkflowDraft) => Promise<WorkflowDraft | undefined>
) {
  const schema = workflowSettingsSchema(saved.name)
  return useForm({
    defaultValues: toDraft(saved),
    validators: { onChange: schema, onBlur: schema, onSubmit: schema },
    onSubmit: async ({ value, formApi }) => {
      const result = await submit(value)
      if (result) formApi.reset(toDraft(result))
    },
  })
}
export type WorkflowForm = ReturnType<typeof useWorkflowForm>
