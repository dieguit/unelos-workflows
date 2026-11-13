import { useCallback } from "react"
import { useStore } from "@tanstack/react-form"
import { PencilIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { SkillPicker } from "@/components/skill-picker"
import { useLazy } from "@/hooks/use-lazy"
import { listAllSkills } from "@/hooks/skill-fetchers"
import type { WorkflowForm } from "../-hooks/use-workflow-form"
import { humanize } from "@/lib/step-edit"
import { StepIconBox } from "@/components/flow/step-card"
import { InputsSection, OutputsSection } from "./step-panel/artifact-sections"
import {
  AgentField,
  ExecutionField,
  NameField,
  useAgents,
} from "./step-panel/basic-fields"
import { PanelBody, PanelHeader, PanelTabs, RemoveStep } from "@/components/step-panel/panel-chrome"
import { Section, SkillChips } from "@/components/step-panel/panel-ui"
import { TaskSourceField } from "./step-panel/task-source-field"
import {
  NamedOutcomes,
  OutcomesSection,
  RoutesSection,
} from "./step-panel/route-sections"

/** Properties panel for the selected task / human-gate step (pencil kSLat). */
export function StepPanel({
  root,
  form,
  index,
  showErrors,
  disabled,
  onEditInstructions,
  nameDraft,
  onNameDraft,
  onClose,
  onRemove,
}: {
  root: string
  form: WorkflowForm
  index: number
  /** Show every field error (after Validate), not just blurred ones. */
  showErrors: boolean
  disabled: boolean
  onEditInstructions: () => void
  nameDraft?: { text: string; error: string }
  onNameDraft: (id: string, draft?: { text: string; error: string }) => void
  onClose: () => void
  onRemove: () => void
}) {
  const steps = useStore(form.store, (s) => s.values.steps)
  const fetchSkills = useCallback(() => listAllSkills(root), [root])
  const skills = useLazy(fetchSkills)
  const agents = useAgents(root)
  const step = steps[index]
  if (!step) return null
  const task = step.kind === "task" ? step.task : null
  const named = task?.source === "named" ? task.id : null
  const props = { form, index, showErrors }
  return (
    <>
      <PanelHeader
        icon={<StepIconBox kind={step.kind} />}
        title={humanize(step.id)}
        subtitle={`${step.kind === "human-gate" ? "Human gate" : "Task step"} · id ${step.id}`}
        onClose={onClose}
      />
      <PanelTabs />
      <PanelBody
        disabled={disabled}
        footer={step.kind !== "end" && <RemoveStep step={step} disabled={disabled} onRemove={onRemove} />}
      >
          <NameField {...props} step={step} nameDraft={nameDraft} onNameDraft={onNameDraft} />
          {step.kind === "task" && (
            <>
              <AgentField {...props} agents={agents} />
              <ExecutionField {...props} />
              <div className="flex flex-col gap-4 border-y py-4">
                {task && <TaskSourceField root={root} form={form} path={`steps[${index}].task`} task={task} />}
                <Section
                  label="Task Instructions"
                  source={named}
                  action={
                    <Button variant="outline" size="sm" onClick={onEditInstructions}>
                      <PencilIcon aria-hidden="true" />
                      Edit instructions
                    </Button>
                  }
                />
                <form.Field name={`steps[${index}].task.skills`} mode="array">
                  {(field) => {
                    const list = field.state.value ?? [] // task is non-null for task steps
                    return (
                      <Section label="Skills" source={named}>
                        <SkillChips skills={list} onRemove={named ? undefined : (i) => field.removeValue(i)} />
                        {named ? (
                          !list.length && (
                            <p className="text-sm text-muted-foreground">None</p>
                          )
                        ) : (
                          <SkillPicker
                            skills={skills.data}
                            error={skills.error}
                            taken={list}
                            onOpen={skills.load}
                            onPick={(skill) => field.pushValue(skill)}
                          />
                        )}
                      </Section>
                    )
                  }}
                </form.Field>
                {named ? (
                  <NamedOutcomes outcomes={task?.outcomes ?? []} source={named} />
                ) : (
                  <OutcomesSection {...props} />
                )}
              </div>
            </>
          )}
          <InputsSection form={form} index={index} steps={steps} />
          {step.kind === "task" && <OutputsSection form={form} index={index} />}
          <RoutesSection {...props} step={step} steps={steps} />
      </PanelBody>
    </>
  )
}
