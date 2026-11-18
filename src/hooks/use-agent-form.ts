import { useForm } from "@tanstack/react-form"
import type { AgentDocument } from "@/lib/projects"
import { agentSchema } from "../-lib/agent-schema"

export const emptyAgent: AgentDocument = {
  id: "",
  instructions: "",
  skills: [],
  mcps: [],
  tools: [],
  model: null,
  effort: null,
}

export function useAgentForm(
  defaultValues: AgentDocument,
  save: (
    value: AgentDocument,
    closeEditor: boolean
  ) => Promise<AgentDocument | undefined>,
  invalid: () => void
) {
  return useForm({
    defaultValues,
    onSubmitMeta: { instructions: null as string | null, closeEditor: false },
    validators: {
      onChange: agentSchema,
      onBlur: agentSchema,
      onSubmit: agentSchema,
    },
    onSubmitInvalid: invalid,
    onSubmit: async ({ value, meta, formApi }) => {
      const saved = await save(
        { ...value, instructions: meta.instructions ?? value.instructions },
        meta.closeEditor
      )
      if (saved) formApi.reset(saved)
    },
  })
}
export type AgentForm = ReturnType<typeof useAgentForm>
