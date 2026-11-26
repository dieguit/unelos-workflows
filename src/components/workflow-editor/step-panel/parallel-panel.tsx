import { useCallback, useEffect } from "react"
import { useStore } from "@tanstack/react-form"
import { ArrowLeftIcon, BotIcon, ChevronRightIcon, LockIcon, PencilIcon, Trash2Icon } from "lucide-react"
import { cn } from "cn"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { SkillPicker } from "@/components/skill-picker"
import { useLazy } from "@/hooks/use-lazy"
import { listSkillChoices } from "@/hooks/skill-fetchers"
import type { WorkflowForm } from "@/hooks/use-workflow-form"
import { addWorker, humanize, slugify, taskRef, workerTitle } from "@/lib/step-edit"
import { StepIconBox } from "@/components/flow/step-card"
import { stepIconColors, stepIcons } from "@/components/flow/step-icon"
import { InputsSection, OutputsSection } from "./artifact-sections"
import { AgentField, NameField, useAgents } from "./basic-fields"
import { PanelBody, PanelHeader, PanelTabs, RemoveStep } from "@/components/step-panel/panel-chrome"
import { ErrorText, InlineAdd, RemoveButton, Row, rowFrame, Section, shown, SkillChips } from "@/components/step-panel/panel-ui"
import { RoutesSection } from "./route-sections"
import { TaskSourceField } from "./task-source-field"

type NameDraft = { text: string; error: string }

/** Properties panel for a selected parallel step (pencil ii2rY). */
export function ParallelPanel({ form, index, showErrors, disabled, nameDraft, onNameDraft, onSelectWorker, onClose, onRemove }: {
  form: WorkflowForm
  index: number
  showErrors: boolean
  disabled: boolean
  nameDraft?: NameDraft
  onNameDraft: (id: string, draft?: NameDraft) => void
  onSelectWorker: (stepIndex: number, workerIndex: number) => void
  onClose: () => void
  onRemove: () => void
}) {
  const steps = useStore(form.store, (s) => s.values.steps)
  const step = steps[index]
  if (!step) return null
  const props = { form, index, showErrors }
  const TaskIcon = stepIcons.task
  return (
    <>
      <PanelHeader
        icon={<StepIconBox kind="parallel" />}
        title={humanize(step.id)}
        subtitle={`Parallel step · id ${step.id}`}
        onClose={onClose}
      />
      <PanelTabs />
      <PanelBody disabled={disabled} footer={<RemoveStep step={step} disabled={disabled} onRemove={onRemove} />}>
        <NameField {...props} step={step} nameDraft={nameDraft} onNameDraft={onNameDraft} />
        <InputsSection form={form} index={index} steps={steps} />
        <Section
          label="Tasks"
          action={<span className="text-xs text-muted-foreground">{step.workers.length}</span>}
        >
          {step.workers.map((worker, i) => {
            const title = workerTitle(worker)
            const incomplete = showErrors && (!title.trim() || !worker.agent.trim() ||
              (worker.task.source === "inline" && !worker.task.instructions.trim()))
            return (
              <div key={i} className="flex flex-col gap-1.5">
                <div className={cn(rowFrame, incomplete && "border-destructive")}>
                  <span className="w-3 shrink-0 text-xs text-muted-foreground tabular-nums">{i + 1}</span>
                  <TaskIcon aria-hidden="true" className={cn("size-3.5 shrink-0", stepIconColors.task)} />
                  <span className={cn("min-w-0 flex-1 truncate", !title && "text-muted-foreground")}>{title || "Untitled task"}</span>
                  <span className="flex h-5 max-w-28 shrink-0 items-center gap-1 rounded-sm bg-muted px-1.5 text-xs text-muted-foreground">
                    <BotIcon aria-hidden="true" className="size-3 shrink-0" />
                    <span className="truncate">{worker.agent || "No agent"}</span>
                  </span>
                  <Button variant="ghost" size="icon-xs" aria-label={`Select task ${i + 1}`} title="Select task" onClick={() => onSelectWorker(index, i)}>
                    <ChevronRightIcon aria-hidden="true" className="text-muted-foreground" />
                  </Button>
                  <RemoveButton
                    label={`Remove task ${i + 1}`}
                    onClick={() => form.setFieldValue(`steps[${index}].workers`, step.workers.filter((_, j) => j !== i))}
                  />
                </div>
                {incomplete && (
                  <ErrorText errors={[{ message: `Task ${i + 1} needs a name, an Agent and instructions. Select it to fix, or remove it.` }]} />
                )}
              </div>
            )
          })}
          {showErrors && !step.workers.length && <ErrorText errors={[{ message: "Add at least one task." }]} />}
          <InlineAdd
            label="Add task"
            onClick={() => {
              form.setFieldValue(`steps[${index}]`, addWorker(step))
              onSelectWorker(index, step.workers.length)
            }}
          />
        </Section>
        <OutputsSection form={form} index={index} />
        <RoutesSection {...props} step={step} steps={steps} />
      </PanelBody>
    </>
  )
}

/** Properties panel for one task inside a parallel step (pencil f3s1m2). */
export function WorkerPanel({ root, form, index, worker: at, showErrors, disabled, onEditInstructions, onBack, onClose, onRemove }: {
  root: string | null
  form: WorkflowForm
  index: number
  worker: number
  showErrors: boolean
  disabled: boolean
  onEditInstructions: () => void
  onBack: () => void
  onClose: () => void
  onRemove: () => void
}) {
  const steps = useStore(form.store, (s) => s.values.steps)
  const fetchSkills = useCallback(() => listSkillChoices(root), [root])
  const skills = useLazy(fetchSkills)
  const agents = useAgents(root)
  const loadAgents = agents.load
  useEffect(loadAgents, [loadAgents])
  const step = steps[index]
  const worker = step?.workers[at]
  if (!step || !worker) return null
  const { task } = worker
  const named = task.source === "named" ? task.id : null
  const title = workerTitle(worker)
  const parent = humanize(step.id)
  return (
    <>
      <div className="flex items-center gap-1.5 border-b px-2 py-1.5 text-sm">
        <Button variant="ghost" size="sm" onClick={onBack}>
          <ArrowLeftIcon aria-hidden="true" /> {parent}
        </Button>
        <ChevronRightIcon aria-hidden="true" className="size-3.5 text-muted-foreground" />
        <span className="text-muted-foreground">Task {at + 1} of {step.workers.length}</span>
      </div>
      <PanelHeader
        icon={<StepIconBox kind="task" />}
        title={title || "Untitled task"}
        subtitle={`Task in ${parent} · id ${named ? taskRef(named).name : slugify(title) || "—"}`}
        tag="Subagent"
        onClose={onClose}
      />
      <PanelTabs />
      <PanelBody
        disabled={disabled}
        footer={
          <Button variant="destructive" size="sm" disabled={disabled} onMouseDown={(e) => e.preventDefault()} onClick={onRemove}>
            <Trash2Icon aria-hidden="true" />
            Remove task
          </Button>
        }
      >
        {!named && (
          <form.Field name={`steps[${index}].workers[${at}].task.description`}>
            {(field) => {
              const errors = shown(field.state.meta, showErrors)
              return (
                <Row label="Name" htmlFor="worker-name" errors={errors} errorId="worker-name-error">
                  <Input
                    id="worker-name"
                    value={field.state.value ?? ""}
                    autoFocus={!field.state.value}
                    spellCheck={false}
                    autoComplete="off"
                    aria-invalid={errors.length > 0}
                    aria-describedby={errors.length ? "worker-name-error" : undefined}
                    onBlur={field.handleBlur}
                    onChange={(e) => field.handleChange(e.target.value)}
                  />
                </Row>
              )
            }}
          </form.Field>
        )}
        <AgentField form={form} index={index} worker={at} agents={agents} showErrors={showErrors} />
        <div className="flex flex-col gap-4 border-y py-4">
          <TaskSourceField root={root} form={form} path={`steps[${index}].workers[${at}].task`} task={task} />
          <Section
            label="Task Instructions"
            source={named}
            action={
              <Button variant="outline" size="sm" onClick={onEditInstructions}>
                <PencilIcon aria-hidden="true" />
                Edit instructions
              </Button>
            }
          >
            <p className="line-clamp-4 rounded-md bg-muted px-2.5 py-2 text-sm whitespace-pre-wrap text-foreground-dim">
              {task.instructions.trim() || <span className="text-muted-foreground">No instructions yet.</span>}
            </p>
            {showErrors && !named && !task.instructions.trim() && <ErrorText errors={[{ message: "Add instructions." }]} />}
          </Section>
          <form.Field name={`steps[${index}].workers[${at}].task.skills`} mode="array">
            {(field) => {
              const list = field.state.value
              return (
                <Section label="Skills" source={named}>
                  <SkillChips skills={list} onRemove={named ? undefined : (i) => field.removeValue(i)} />
                  {named ? (
                    !list.length && <p className="text-sm text-muted-foreground">None</p>
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
        </div>
        <Section
          label="Input"
          action={<span className="text-xs text-muted-foreground">shared from parent</span>}
        >
          {step.inputs.length ? (
            step.inputs.map((artifact) => (
              <div key={artifact} className={cn(rowFrame, "border-transparent bg-muted pr-2.5")}>
                <LockIcon aria-hidden="true" className="size-3.5 shrink-0 text-muted-foreground" />
                <span className="min-w-0 flex-1 truncate font-mono text-xs text-foreground-dim">{artifact}</span>
              </div>
            ))
          ) : (
            <p className="text-sm text-muted-foreground">None bound on {parent}.</p>
          )}
        </Section>
      </PanelBody>
    </>
  )
}
