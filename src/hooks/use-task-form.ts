import { useForm } from "@tanstack/react-form"
import type { TaskDocument } from "@/lib/projects"
import { taskSchema } from "@/lib/task-schema"

export const emptyTask: TaskDocument = {
  id: "",
  description: null,
  instructions: "",
  skills: [],
  outcomes: [],
}

export function useTaskForm(
  defaultValues: TaskDocument,
  save: (
    value: TaskDocument,
    closeEditor: boolean
  ) => Promise<TaskDocument | undefined>,
  invalid: () => void
) {
  return useForm({
    defaultValues,
    onSubmitMeta: { instructions: null as string | null, closeEditor: false },
    validators: {
      onChange: taskSchema,
      onBlur: taskSchema,
      onSubmit: taskSchema,
    },
    onSubmitInvalid: invalid,
    onSubmit: async ({ value, meta, formApi }) => {
      const saved = await save(
        {
          ...value,
          description: value.description?.trim() || null,
          instructions: meta.instructions ?? value.instructions,
        },
        meta.closeEditor
      )
      if (saved) formApi.reset(saved)
    },
  })
}
export type TaskForm = ReturnType<typeof useTaskForm>
