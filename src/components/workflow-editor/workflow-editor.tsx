import { BusyContent } from "@/components/busy-content"
import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import { useEffect, useRef, useState } from "react"
import { useStore } from "@tanstack/react-form"
import { useBlocker, useRouter } from "@tanstack/react-router"
import {
  ArrowLeftIcon,
  CheckCircle2Icon,
  ChevronDownIcon,
  CircleAlertIcon,
  ListChecksIcon,
  PanelRightCloseIcon,
  SaveIcon,
  SlidersHorizontalIcon,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { useDiscardChanges } from "@/hooks/use-discard-changes"
import { registerProjectChangeGuard } from "@/lib/project-change-guard"
import {
  getGlobalWorkflow,
  getWorkflow,
  saveGlobalWorkflow,
  saveWorkflow,
  validateGlobalWorkflow,
  validateWorkflow,
  type WorkflowDocument,
  type WorkflowDraft,
  type WorkflowIssue,
  type WorkflowStep,
} from "@/lib/projects"
import { globalRef } from "@/lib/global-ref"
import { BodyEditor } from "@/components/body-editor"
import { applyBodyDraft } from "@/lib/apply-body-draft"
import { addWorker, humanize, insertStep, removeStep, withDerivedOutcomes, workerTitle } from "@/lib/step-edit"
import { workflowSettingsSchema } from "@/lib/workflow-settings-schema"
import { useWorkflowForm } from "@/hooks/use-workflow-form"
import { FlowPane } from "@/components/flow/flow-pane"
import { FlowToolbar } from "@/components/flow/flow-toolbar"
import { StepPicker } from "./step-picker"
import { StepPanel } from "./step-panel"
import { ParallelPanel, WorkerPanel } from "./step-panel/parallel-panel"
import { WorkflowCallPanel } from "./step-panel/workflow-call-panel"
import { WorkflowInputsSection } from "./step-panel/artifact-sections"

/** `global` edits the machine-wide definition in ~/.config/unelos; `root` is then only the open project, if any. */
export function WorkflowEditor({
  root,
  id,
  global = false,
  onBack,
}: {
  root: string | null
  id: string
  global?: boolean
  onBack: () => void
}) {
  const [document, setDocument] = useState<WorkflowDocument>()
  const [error, setError] = useState<string>()
  const [attempt, setAttempt] = useState(0)
  useEffect(() => {
    let active = true
    ;(global ? getGlobalWorkflow(id) : getWorkflow(root!, id)).then(
      (doc) => {
        if (active) {
          setDocument(doc)
          setError(undefined)
        }
      },
      (err) => {
        if (active) setError(String(err))
      }
    )
    return () => {
      active = false
    }
  }, [root, id, global, attempt])
  if (document)
    return (
      <WorkflowEditorForm root={root} global={global} document={document} onBack={onBack} />
    )
  return (
    <div className="flex flex-col items-start gap-4">
      <Button variant="ghost" onClick={onBack}>
        <ArrowLeftIcon /> {global ? "Global workflows" : "Workflows"}
      </Button>
      {error ? (
        <div role="alert" className="space-y-3">
          <p>Could not load workflow: {error}</p>
          <Button
            variant="outline"
            onClick={() => {
              setError(undefined)
              setAttempt((n) => n + 1)
            }}
          >
            Retry
          </Button>
        </div>
      ) : (
        <p role="status">Loading workflow…</p>
      )}
    </div>
  )
}

function WorkflowEditorForm({
  root,
  global,
  document,
  onBack,
}: {
  root: string | null
  global: boolean
  document: WorkflowDocument
  onBack: () => void
}) {
  const router = useRouter()
  const discard = useDiscardChanges()
  const [panel, setPanel] = useState(true)
  const [saved, setSaved] = useState(document)
  const [saveError, setSaveError] = useState<string>()
  const [validation, setValidation] = useState<{
    value: WorkflowDraft
    id: string
    issues: WorkflowIssue[]
    error?: string
  }>()
  const [validationAttempt, setValidationAttempt] = useState(0)
  const [selected, setSelected] = useState<number | null>(null)
  const [selectedWorker, setSelectedWorker] = useState<number | null>(null)
  // Remounts the step panel per selection so its local name text and autofocus start fresh.
  const [selection, setSelection] = useState(0)
  // Instructions editor buffer for the selected step; replaces the flow while open.
  const [instructions, setInstructions] = useState<string | null>(null)
  const [nameDrafts, setNameDrafts] = useState<Record<string, { text: string; error: string }>>({})
  // Pickers in a global workflow may only offer global items.
  const pickerRoot = global ? null : root
  const schema = workflowSettingsSchema(saved.name)
  const form = useWorkflowForm(saved, async (value) => {
    setSaveError(undefined)
    try {
      const body = { ...value, steps: withDerivedOutcomes(value.steps) }
      const result = global
        ? await saveGlobalWorkflow(root, saved.id, body)
        : await saveWorkflow(root!, saved.id, body)
      setSaved(result)
      void router.invalidate()
      return result
    } catch (err) {
      setSaveError(String(err))
      setSelected(null)
      setSelectedWorker(null)
      setInstructions(null)
      setPanel(true)
    }
  })
  const draft = useStore(form.store, (s) => s.values)
  const dirty = !useStore(form.store, (s) => s.isDefaultValue) || Object.keys(nameDrafts).length > 0
  const saving = useStore(form.store, (s) => s.isSubmitting)
  const visualSaving = useDelayedFlag(saving)
  const localResult = schema.safeParse(draft)
  const localValid = localResult.success
  const step = selected === null ? undefined : draft.steps[selected]
  const worker = selectedWorker === null ? undefined : step?.workers[selectedWorker]
  const selectedTask = selectedWorker === null ? step?.task : worker?.task
  const instructionsDirty = instructions !== null && instructions !== (selectedTask?.instructions ?? "")
  // Just-inserted step: its issues stay hidden until it is edited, left, or validated.
  const [pristine, setPristine] = useState<WorkflowStep | null>(null)
  const select = (index: number | null) => {
    setPristine(null)
    setSelected(index)
    setSelectedWorker(null)
    setSelection((n) => n + 1)
    setInstructions(null)
    if (index !== null) setPanel(true)
  }
  const closeStep = () => {
    if (instructions !== null) return
    select(null)
    setPanel(false)
  }
  const selectWorker = (stepIndex: number, workerIndex: number | null) => {
    select(stepIndex)
    setSelectedWorker(workerIndex)
  }
  const addTask = (stepIndex: number) => {
    const target = form.state.values.steps[stepIndex]
    form.setFieldValue(`steps[${stepIndex}]`, addWorker(target))
    selectWorker(stepIndex, target.workers.length)
  }
  const removeSelectedWorker = () => {
    if (selected === null || selectedWorker === null) return
    form.setFieldValue(`steps[${selected}].workers`, form.state.values.steps[selected].workers.filter((_, i) => i !== selectedWorker))
    select(selected)
  }
  const editInstructions = () => setInstructions(selectedTask?.instructions ?? "")
  const addStep = (index: number, kind: "task" | "human-gate" | "workflow" | "parallel") => {
    const result = insertStep(form.state.values.steps, index, kind)
    form.setFieldValue("steps", result.steps)
    select(result.index)
    setPristine(result.steps[result.index])
  }
  const onNameDraft = (id: string, value?: { text: string; error: string }) =>
    setNameDrafts((drafts) => {
      const next = { ...drafts }
      if (value) next[id] = value
      else delete next[id]
      return next
    })
  const removeSelectedStep = () => {
    if (!step || saving) return
    const result = removeStep(form.state.values.steps, form.state.values.start, step.id)
    form.setFieldValue("steps", result.steps)
    form.setFieldValue("start", result.start)
    setNameDrafts((drafts) => {
      const next = { ...drafts }
      delete next[step.id]
      return next
    })
    select(null)
    setValidationAttempt((n) => n + 1)
  }
  useEffect(() => {
    let active = true
    const body = { ...draft, steps: withDerivedOutcomes(draft.steps) }
    ;(global ? validateGlobalWorkflow(saved.id, body) : validateWorkflow(root!, saved.id, body)).then(
      (issues) => {
        if (active) setValidation({ value: draft, id: saved.id, issues })
      },
      (err) => {
        if (active)
          setValidation({
            value: draft,
            id: saved.id,
            issues: [],
            error: String(err),
          })
      }
    )
    return () => {
      active = false
    }
  }, [root, global, saved.id, draft, validationAttempt])
  const current =
    validation?.value === draft && validation.id === saved.id
      ? validation
      : undefined
  const issues = [
    ...new Map(
      [
        ...(localResult.success
          ? []
          : localResult.error.issues.map((issue): WorkflowIssue => ({
              message: issue.message,
              step:
                issue.path[0] === "steps"
                  ? (draft.steps[issue.path[1] as number]?.id ?? null)
                  : null,
            }))),
        // Core issues for a draft that fails the form schema just restate those errors in YAML terms.
        ...(localResult.success ? (current?.issues ?? []) : []),
        ...Object.entries(nameDrafts).map(([step, draft]): WorkflowIssue => ({ step, message: draft.error })),
      ].map((issue) => [`${issue.step}:${issue.message}`, issue])
    ).values(),
  ].filter(
    (issue) =>
      !pristine ||
      !!nameDrafts[issue.step ?? ""] ||
      issue.step !== pristine.id ||
      !draft.steps.includes(pristine)
  )
  const issueCounts = new Map<string, number>()
  for (const { step } of issues)
    if (step) issueCounts.set(step, (issueCounts.get(step) ?? 0) + 1)
  const [highlight, setHighlight] = useState<string>()
  useEffect(() => {
    if (!highlight) return
    window.document
      .getElementById(`flow-step-${highlight}`)
      ?.scrollIntoView({ block: "center", behavior: "smooth" })
    const timer = setTimeout(() => setHighlight(undefined), 1600)
    return () => clearTimeout(timer)
  }, [highlight])
  const valid =
    localValid && Object.keys(nameDrafts).length === 0 && current && !current.error &&
    !current.issues.some((issue) => issue.severity !== "warning")
  const askDiscard = discard.ask
  const projectChangeApproved = useRef(false)
  useEffect(
    () =>
      registerProjectChangeGuard(async () => {
        if (saving || ((dirty || instructionsDirty) && !(await askDiscard()))) return null
        projectChangeApproved.current = true
        return () => {
          projectChangeApproved.current = false
        }
      }),
    [saving, dirty, instructionsDirty, askDiscard]
  )
  useBlocker({
    shouldBlockFn: async () =>
      !projectChangeApproved.current &&
      (saving || ((dirty || instructionsDirty) && !(await askDiscard()))),
    enableBeforeUnload: dirty || instructionsDirty || saving,
  })
  useEffect(() => {
    const close = (e: KeyboardEvent) => {
      if (
        e.key === "Escape" &&
        !e.defaultPrevented &&
        !window.document.querySelector(
          '[role="dialog"], [data-slot="popover-content"], [data-slot="dropdown-menu-content"]'
        )
      )
        setPanel(false)
    }
    window.addEventListener("keydown", close)
    return () => window.removeEventListener("keydown", close)
  }, [])
  const back = async () => {
    if (!saving && (!(dirty || instructionsDirty) || (await askDiscard()))) onBack()
  }
  return (
    <div inert={saving} className="-mx-8 -mt-6 -mb-8 flex h-[calc(100%+3.5rem)] min-h-0 flex-col">
      <header className="flex min-h-14 shrink-0 items-center gap-3 overflow-x-auto border-b px-3 py-2">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void back()}
          disabled={visualSaving}
        >
          <ArrowLeftIcon /> {global ? "Global workflows" : "Workflows"}
        </Button>
        <div className="h-6 border-l" />
        <div className="flex min-w-0 items-baseline gap-2">
          <h1 className="truncate text-lg font-semibold">{saved.name}</h1>
          <span className="truncate font-mono text-xs text-muted-foreground">
            {global ? `global/${saved.id}.yaml` : `${saved.id}.yaml`}
          </span>
          {global && (
            <span className="shrink-0 rounded-sm bg-primary-muted px-1.5 py-0.5 text-xs font-medium text-primary">
              Global
            </span>
          )}
        </div>
        {dirty && (
          <span className="flex shrink-0 items-center gap-1.5 rounded-sm bg-warning-muted px-2 py-0.5 text-xs text-warning">
            <span className="size-1.5 rounded-full bg-warning" />
            Unsaved changes
          </span>
        )}
        <div className="flex-1" />
        {!current ? (
          <span role="status" className="text-xs text-muted-foreground">
            Validating…
          </span>
        ) : current.error ? (
          <span role="alert" className="text-xs text-destructive">
            Validation failed
          </span>
        ) : issues.length ? (
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button
                  variant="outline"
                  size="sm"
                  className={issues.every((issue) => issue.severity === "warning")
                    ? "border-warning/50 text-warning hover:text-warning"
                    : "border-destructive/50 text-destructive hover:text-destructive"}
                />
              }
            >
              <CircleAlertIcon /> {issues.length}{" "}
              {issues.length === 1 ? "issue" : "issues"}
              <ChevronDownIcon />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-80">
              {issues.map((issue) => (
                <DropdownMenuItem
                  key={issue.message}
                  className="items-start"
                  onClick={() => issue.step && setHighlight(issue.step)}
                >
                  <CircleAlertIcon className={issue.severity === "warning" ? "mt-0.5 text-warning" : "mt-0.5 text-destructive"} />
                  <span className="min-w-0 flex-1">
                    {issue.severity === "warning" && "Warning: "}{issue.message}
                    {issue.step && (
                      <span className="block font-mono text-xs text-muted-foreground">
                        {issue.step}
                      </span>
                    )}
                  </span>
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        ) : (
          <span className="flex items-center gap-1 text-xs text-muted-foreground">
            <CheckCircle2Icon /> Valid
          </span>
        )}
        <Button
          variant={panel && !step ? "secondary" : "ghost"}
          size="sm"
          disabled={instructions !== null}
          onClick={() => (step ? select(null) : setPanel(!panel))}
          aria-pressed={panel && !step}
        >
          <SlidersHorizontalIcon /> Workflow settings
        </Button>
        <div className="h-6 border-l" />
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setValidation(undefined)
            setValidationAttempt((n) => n + 1)
            setPristine(null)
          }}
          disabled={visualSaving}
        >
          <ListChecksIcon /> Validate
        </Button>
        <Button
          size="sm"
          onClick={() => {
            if (dirty && valid && !saving && instructions === null)
              void form.handleSubmit()
            else if (dirty && !saving && instructions === null) {
              setValidationAttempt((n) => n + 1)
              setPristine(null)
            }
          }}
          disabled={!dirty || visualSaving || instructions !== null}
        >
          <BusyContent busy={visualSaving}><SaveIcon /> Save</BusyContent>
        </Button>
      </header>
      <div className="relative flex min-h-0 flex-1">
        {step && instructions !== null ? (
          <div className="flex min-w-0 flex-1 flex-col p-4">
            <BodyEditor
              title={`${worker ? workerTitle(worker) : humanize(step.id)} instructions`}
              label="Instructions"
              body={instructions}
              onChange={setInstructions}
              onCancel={() => {
                void (async () => {
                  if (!instructionsDirty || await askDiscard("Discard unsaved changes to instructions? The workflow draft will be kept."))
                    setInstructions(null)
                })()
              }}
              onSave={async () => {
                applyBodyDraft(instructions, (body) => {
                  if (selectedWorker === null) form.setFieldValue(`steps[${selected!}].task.instructions`, body)
                  else form.setFieldValue(`steps[${selected!}].workers[${selectedWorker}].task.instructions`, body)
                }, () => setInstructions(null))
              }}
            />
          </div>
        ) : (
        <div className="flex min-w-0 flex-1 flex-col">
          <FlowToolbar
            startPicker={
              <form.Field name="start">
                {(field) => (
                  <div className="flex shrink-0 items-center gap-2">
                    <label
                      htmlFor="flow-start"
                      className="text-muted-foreground"
                    >
                      Start step
                    </label>
                    <StepPicker
                      id="flow-start"
                      variant="compact"
                      steps={draft.steps}
                      value={field.state.value}
                      onChange={field.handleChange}
                      onBlur={field.handleBlur}
                      disabled={visualSaving}
                      invalid={
                        (field.state.meta.isBlurred || validationAttempt > 0) && !field.state.meta.isValid
                      }
                    />
                  </div>
                )}
              </form.Field>
            }
          />
          <FlowPane
            steps={draft.steps}
            start={draft.start}
            issueCounts={issueCounts}
            highlight={highlight}
            selected={step?.id}
            selectedWorker={selectedWorker}
            onAddTask={visualSaving ? undefined : addTask}
            onSelect={(id) => select(draft.steps.findIndex((s) => s.id === id))}
            onSelectWorker={selectWorker}
            onAdd={visualSaving ? undefined : addStep}
          />
        </div>
        )}
        <aside
          aria-label={step ? `Step ${humanize(step.id)}` : "Workflow settings"}
          hidden={!panel}
          className="hidden:!hidden absolute inset-y-0 right-0 z-10 flex w-[380px] max-w-full flex-col border-l bg-background shadow-lg min-[1280px]:relative min-[1280px]:shrink-0 min-[1280px]:shadow-none"
        >
          {step && selected !== null && step.kind === "parallel" ? (
            selectedWorker === null ? (
              <ParallelPanel
                key={selection}
                form={form}
                index={selected}
                showErrors={validationAttempt > 0}
                disabled={visualSaving}
                nameDraft={nameDrafts[step.id]}
                onNameDraft={onNameDraft}
                onSelectWorker={selectWorker}
                onRemove={removeSelectedStep}
                onClose={closeStep}
              />
            ) : (
              <WorkerPanel
                key={selection}
                root={pickerRoot}
                form={form}
                index={selected}
                worker={selectedWorker}
                showErrors={validationAttempt > 0}
                disabled={visualSaving}
                onEditInstructions={editInstructions}
                onBack={() => { if (instructions === null) select(selected) }}
                onRemove={removeSelectedWorker}
                onClose={closeStep}
              />
            )
          ) : step && selected !== null && step.kind === "workflow" ? (
            <WorkflowCallPanel
              key={selection}
              root={pickerRoot}
              current={global ? globalRef(saved.id) : saved.id}
              form={form}
              index={selected}
              showErrors={validationAttempt > 0}
              disabled={visualSaving}
              nameDraft={nameDrafts[step.id]}
              onNameDraft={onNameDraft}
              onRemove={removeSelectedStep}
              onClose={closeStep}
            />
          ) : step && selected !== null ? (
            <StepPanel
              key={selection}
              root={pickerRoot}
              form={form}
              index={selected}
              showErrors={validationAttempt > 0}
              disabled={visualSaving}
              nameDraft={nameDrafts[step.id]}
              onNameDraft={onNameDraft}
              onEditInstructions={editInstructions}
              onRemove={removeSelectedStep}
              onClose={closeStep}
            />
          ) : (
          <>
          <header className="flex items-center gap-2.5 py-3 pr-2 pl-4">
            <span className="flex size-7 items-center justify-center rounded-md bg-muted text-muted-foreground">
              <SlidersHorizontalIcon className="size-4" />
            </span>
            <h2 className="flex-1 text-lg font-semibold">Workflow settings</h2>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Close settings"
              onClick={() => setPanel(false)}
            >
              <PanelRightCloseIcon />
            </Button>
          </header>
          <div className="overflow-y-auto p-4">
            <form
              id="workflow-settings-form"
              noValidate
              onSubmit={(e) => {
                e.preventDefault()
                if (dirty && valid && !saving) void form.handleSubmit()
              }}
            >
              <fieldset
                disabled={visualSaving}
                className="min-w-0 disabled:opacity-60"
              >
                <FieldGroup className="gap-4">
                  <WorkflowInputsSection form={form} />
                  <form.Field name="name">
                    {(field) => (
                      <Field
                        className="grid grid-cols-[76px_minmax(0,1fr)] items-center gap-x-2 [&>label]:text-foreground-dim [&>[data-slot=field-description]]:col-start-2 [&>[data-slot=field-error]]:col-start-2"
                        data-invalid={
                           (field.state.meta.isBlurred || validationAttempt > 0) &&
                          !field.state.meta.isValid
                        }
                      >
                        <FieldLabel htmlFor="workflow-name">Name</FieldLabel>
                        <Input
                          id="workflow-name"
                          name={field.name}
                          required
                          value={field.state.value}
                          onBlur={field.handleBlur}
                          onChange={(e) => {
                            field.handleChange(e.target.value)
                            if (field.state.meta.isBlurred) field.handleBlur()
                          }}
                          aria-invalid={
                            (field.state.meta.isBlurred || validationAttempt > 0) &&
                            !field.state.meta.isValid
                          }
                          aria-describedby={
                            (field.state.meta.isBlurred || validationAttempt > 0) &&
                            !field.state.meta.isValid
                              ? "workflow-name-help workflow-name-error"
                              : "workflow-name-help"
                          }
                        />
                        <FieldDescription id="workflow-name-help">
                          Changing the name also renames the workflow file.
                        </FieldDescription>
                        <FieldError
                          id="workflow-name-error"
                          errors={
                            field.state.meta.isBlurred || validationAttempt > 0
                              ? field.state.meta.errors
                              : []
                          }
                        />
                      </Field>
                    )}
                  </form.Field>
                  <form.Field name="description">
                    {(field) => (
                      <Field className="grid grid-cols-[76px_minmax(0,1fr)] items-start gap-x-2 [&>label]:pt-2 [&>label]:text-foreground-dim">
                        <FieldLabel htmlFor="workflow-description">
                          Description
                        </FieldLabel>
                        <Textarea
                          id="workflow-description"
                          name={field.name}
                          value={field.state.value ?? ""}
                          onBlur={field.handleBlur}
                          onChange={(e) =>
                            field.handleChange(e.target.value || null)
                          }
                        />
                      </Field>
                    )}
                  </form.Field>
                  <form.Field name="start">
                    {(field) => (
                      <Field
                        className="grid grid-cols-[76px_minmax(0,1fr)] items-center gap-x-2 [&>label]:text-foreground-dim [&>[data-slot=field-description]]:col-start-2 [&>[data-slot=field-error]]:col-start-2"
                        data-invalid={
                          (field.state.meta.isBlurred || validationAttempt > 0) &&
                          !field.state.meta.isValid
                        }
                      >
                        <FieldLabel htmlFor="workflow-start">
                          Start step
                        </FieldLabel>
                        <StepPicker
                          id="workflow-start"
                          variant="field"
                          steps={draft.steps}
                          value={field.state.value}
                          onChange={field.handleChange}
                          onBlur={field.handleBlur}
                          invalid={
                            (field.state.meta.isBlurred || validationAttempt > 0) &&
                            !field.state.meta.isValid
                          }
                          describedBy={
                            (field.state.meta.isBlurred || validationAttempt > 0) &&
                            !field.state.meta.isValid
                              ? "workflow-start-help workflow-start-error"
                              : "workflow-start-help"
                          }
                        />
                        <FieldDescription id="workflow-start-help">
                          Runs begin at this step. It controls entry
                          independently of where the step sits in the flow.
                        </FieldDescription>
                        <FieldError
                          id="workflow-start-error"
                          errors={
                            field.state.meta.isBlurred || validationAttempt > 0
                              ? field.state.meta.errors
                              : []
                          }
                        />
                      </Field>
                    )}
                  </form.Field>
                </FieldGroup>
              </fieldset>
              {current?.error && (
                <p role="alert" className="mt-4 text-sm text-destructive">
                  Could not validate workflow: {current.error}
                </p>
              )}
              {saveError && (
                <p role="alert" className="mt-4 text-sm text-destructive">
                  Could not save workflow: {saveError}
                </p>
              )}
            </form>
          </div>
          </>
          )}
        </aside>
      </div>
      {discard.dialog}
    </div>
  )
}
