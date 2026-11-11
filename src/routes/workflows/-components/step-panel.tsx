import { useCallback } from "react"
import { useStore } from "@tanstack/react-form"
import { PanelRightCloseIcon, PencilIcon } from "lucide-react"
import { cn } from "cn"
import { Button } from "@/components/ui/button"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { SkillPicker } from "@/components/skill-picker"
import { useLazy } from "@/hooks/use-lazy"
import { listSkills } from "@/lib/projects"
import type { WorkflowForm } from "../-hooks/use-workflow-form"
import { humanize } from "../-lib/step-edit"
import { StepIconBox } from "./flow/step-card"
import { InputsSection, OutputsSection } from "./step-panel/artifact-sections"
import {
  AgentField,
  ExecutionField,
  NameField,
} from "./step-panel/basic-fields"
import { RemoveButton, Section } from "./step-panel/panel-ui"
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
}) {
  const steps = useStore(form.store, (s) => s.values.steps)
  const fetchSkills = useCallback(() => listSkills(root), [root])
  const skills = useLazy(fetchSkills)
  const step = steps[index]
  if (!step) return null
  const task = step.kind === "task" ? step.task : null
  const named = task?.source === "named" ? task.id : null
  const props = { form, index, showErrors }
  return (
    <>
      <header className="flex items-center gap-2.5 py-3 pr-2 pl-4">
        <StepIconBox kind={step.kind} />
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-lg font-semibold">
            {humanize(step.id)}
          </h2>
          <p className="truncate font-mono text-xs text-muted-foreground">
            {step.kind === "human-gate" ? "Human gate" : "Task step"} · id{" "}
            {step.id}
          </p>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Close step"
          title="Close step"
          onClick={onClose}
        >
          <PanelRightCloseIcon aria-hidden="true" />
        </Button>
      </header>
      <div
        role="tablist"
        aria-label="Step views"
        className="flex gap-4.5 border-b px-4 text-sm"
      >
        <button
          type="button"
          role="tab"
          id="step-tab-properties"
          aria-selected="true"
          aria-controls="step-tab-panel"
          className="-mb-px h-8.5 border-b-2 border-primary font-medium"
        >
          Properties
        </button>
        <Tooltip>
          <TooltipTrigger
            render={
              <button
                type="button"
                role="tab"
                aria-selected="false"
                aria-disabled="true"
                className="h-8.5 cursor-not-allowed text-muted-foreground"
              />
            }
          >
            Context preview
          </TooltipTrigger>
          <TooltipContent>Coming soon</TooltipContent>
        </Tooltip>
      </div>
      <div
        id="step-tab-panel"
        role="tabpanel"
        aria-labelledby="step-tab-properties"
        className="min-h-0 flex-1 overflow-y-auto"
      >
        <fieldset
          disabled={disabled}
          className="flex min-w-0 flex-col gap-4 p-4 disabled:opacity-60"
        >
          <NameField {...props} step={step} nameDraft={nameDraft} onNameDraft={onNameDraft} />
          {step.kind === "task" && (
            <>
              <AgentField {...props} root={root} />
              <ExecutionField {...props} />
              <div className="flex flex-col gap-4 border-y py-4">
                {task && <TaskSourceField root={root} form={form} index={index} task={task} />}
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
                        {list.length > 0 && (
                          <ul className="flex flex-wrap gap-1.5">
                            {list.map((skill, i) => (
                              <li
                                key={skill}
                                className={cn(
                                  "flex h-7 items-center gap-1 rounded-md border border-input pr-0.5 pl-1.5 font-mono text-xs",
                                  named && "pr-1.5"
                                )}
                              >
                                {skill}
                                {!named && (
                                  <RemoveButton
                                    label={`Remove ${skill}`}
                                    onClick={() => field.removeValue(i)}
                                  />
                                )}
                              </li>
                            ))}
                          </ul>
                        )}
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
        </fieldset>
      </div>
    </>
  )
}
