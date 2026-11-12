import { BusyContent } from "@/components/busy-content"
import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import { useCallback } from "react"
import { useStore } from "@tanstack/react-form"
import { PanelRightCloseIcon, PencilIcon, PlusIcon, XIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
  FieldTitle,
} from "@/components/ui/field"
import type { SkillSummary, TaskDocument } from "@/lib/projects"
import { useLazy } from "@/hooks/use-lazy"
import { SkillPicker } from "@/components/skill-picker"
import type { TaskForm } from "@/hooks/use-task-form"

export function TaskInspector({
  fetchSkills,
  scope,
  originalId,
  form,
  draft,
  dirty,
  loading,
  loadError,
  saveError,
  saving,
  editing,
  onEdit,
  onClose,
  onRetry,
  onOutcomeAdd,
  onOutcomeRemove,
}: {
  /** Skills offered by the picker; null when no project is open to pick from. */
  fetchSkills: (() => Promise<SkillSummary[]>) | null
  /** Shown under the title, e.g. "tasks/review" or "Project Task". */
  scope: { path: string; label: string }
  originalId: string | null
  form: TaskForm
  draft?: TaskDocument
  dirty: boolean
  loading: boolean
  loadError?: string
  saveError?: string
  saving: boolean
  editing: boolean
  onEdit: () => void
  onClose: () => void
  onRetry: () => void
  onOutcomeAdd: () => void
  onOutcomeRemove: (index: number) => void
}) {
  const visualSaving = useDelayedFlag(saving)
  const noSkills = useCallback(() => Promise.resolve([] as SkillSummary[]), [])
  const skills = useLazy(fetchSkills ?? noSkills)
  const attempted = useStore(form.store, (s) => s.submissionAttempts > 0)
  return (
    <aside inert={saving}
      aria-label={originalId ? `Task ${originalId}` : "New Task"}
      className="flex w-[380px] max-w-full shrink-0 flex-col border-l bg-background"
    >
      <header className="flex min-h-16 shrink-0 items-center gap-2 px-4 py-3">
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-base font-semibold">
            {originalId ?? "New Task"}
          </h2>
          <p className="truncate font-mono text-xs text-muted-foreground">
            {originalId ? `${scope.path}/${originalId}` : scope.label}
          </p>
          {dirty ? (
            <p role="status" className="mt-1 text-xs text-warning">
              Unsaved changes
            </p>
          ) : null}
        </div>
        <Button
          size="sm"
          type="submit"
          form="task-form"
          disabled={!dirty || visualSaving || editing}
        >
          <BusyContent busy={visualSaving}>Save</BusyContent>
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Close panel"
          title="Close panel"
          disabled={visualSaving}
          onClick={onClose}
        >
          <PanelRightCloseIcon aria-hidden="true" />
        </Button>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {loading ? (
          <p role="status" className="text-sm text-muted-foreground">
            Loading Task…
          </p>
        ) : null}
        {loadError ? (
          <div role="alert" className="flex flex-col items-start gap-3 text-sm">
            <p className="[overflow-wrap:anywhere] text-destructive">
              Could not load Task: {loadError}
            </p>
            <Button variant="outline" size="sm" onClick={onRetry}>
              Retry
            </Button>
          </div>
        ) : null}
        {draft ? (
          <form
            id="task-form"
            noValidate
            onSubmit={(e) => {
              e.preventDefault()
              if (dirty && !saving && !editing) void form.handleSubmit()
            }}
          >
            <fieldset
              disabled={visualSaving || editing}
              className="min-w-0 disabled:opacity-60"
            >
              <FieldGroup className="gap-4">
                {!originalId && (
                  <form.Field name="id">
                    {(field) => {
                      const invalid =
                        (field.state.meta.isBlurred || attempted) &&
                        !field.state.meta.isValid
                      return (
                        <Field data-invalid={invalid} className="grid grid-cols-[76px_minmax(0,1fr)] items-center gap-x-2 [&>label]:text-foreground-dim [&>div]:col-start-2">
                          <FieldLabel htmlFor="task-id">Name (id)</FieldLabel>
                          <Input
                            id="task-id"
                            name={field.name}
                            value={field.state.value}
                            required
                            autoFocus
                            spellCheck={false}
                            autoComplete="off"
                            placeholder="e.g. code-review"
                            onBlur={field.handleBlur}
                            onChange={(e) => {
                              field.handleChange(e.target.value)
                              if (field.state.meta.isBlurred) field.handleBlur()
                            }}
                            aria-invalid={invalid}
                            aria-describedby={
                              invalid ? "task-id-error" : undefined
                            }
                            className="h-9 font-mono font-normal"
                          />
                          <div>
                            {invalid ? (
                              <FieldError
                                id="task-id-error"
                                errors={field.state.meta.errors}
                              />
                            ) : null}
                          </div>
                        </Field>
                      )
                    }}
                  </form.Field>
                )}
                <form.Field name="description">
                  {(field) => (
                    <Field className="grid grid-cols-[76px_minmax(0,1fr)] items-center gap-x-2 [&>label]:text-foreground-dim">
                      <FieldLabel htmlFor="task-description">
                        Description
                      </FieldLabel>
                      <Input
                        id="task-description"
                        value={field.state.value ?? ""}
                        autoComplete="off"
                        placeholder="What this task does"
                        onBlur={field.handleBlur}
                        onChange={(e) =>
                          field.handleChange(e.target.value || null)
                        }
                        className="h-9 font-normal"
                      />
                    </Field>
                  )}
                </form.Field>
                <form.Field name="instructions">
                  {(field) => (
                    <Field>
                      <div className="flex items-center justify-between gap-2">
                        <FieldTitle>Instructions</FieldTitle>
                        <Button variant="outline" size="sm" onClick={onEdit}>
                          <PencilIcon aria-hidden="true" />
                          Edit instructions
                        </Button>
                      </div>
                      <pre className="line-clamp-5 rounded-lg border bg-card p-3 font-mono text-xs leading-relaxed [overflow-wrap:anywhere] whitespace-pre-wrap text-foreground-dim">
                        {field.state.value.trim() || "Empty"}
                      </pre>
                    </Field>
                  )}
                </form.Field>
                <form.Field name="skills" mode="array">
                  {(field) => (
                    <Field onBlur={field.handleBlur}>
                      <FieldTitle>Skills</FieldTitle>
                      {field.state.value.length ? (
                        <ul className="flex flex-wrap gap-1.5">
                          {field.state.value.map((skill, i) => (
                            <li
                              key={`${i}-${skill}`}
                              className="flex h-7 min-w-0 items-center gap-1 rounded-md border pr-0.5 pl-1.5 font-mono text-xs"
                            >
                              <span
                                className="min-w-0 flex-1 truncate text-sm"
                                title={skill}
                              >
                                {skill}
                              </span>
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                aria-label={`Remove ${skill}`}
                                onClick={() => field.removeValue(i)}
                              >
                                <XIcon aria-hidden="true" />
                              </Button>
                            </li>
                          ))}
                        </ul>
                      ) : (
                        <p className="text-sm text-muted-foreground">
                          Not configured
                        </p>
                      )}
                      {fetchSkills ? (
                        <SkillPicker
                          skills={skills.data}
                          error={skills.error}
                          taken={field.state.value}
                          onOpen={skills.load}
                          onPick={(skill) => field.pushValue(skill)}
                        />
                      ) : (
                        <p className="text-xs text-muted-foreground">
                          Open a project to add skills from it.
                        </p>
                      )}
                    </Field>
                  )}
                </form.Field>
                <form.Field name="outcomes" mode="array">
                  {(field) => (
                    <Field>
                      <FieldTitle>Outcomes</FieldTitle>
                      <FieldDescription>
                        Each outcome becomes a route in workflows that use this
                        task. Leave empty for a single next step.
                      </FieldDescription>
                      {field.state.value.map((_, i) => (
                        <form.Field key={i} name={`outcomes[${i}]`}>
                          {(outcome) => {
                            const invalid =
                              (outcome.state.meta.isBlurred || attempted) &&
                              !outcome.state.meta.isValid
                            return (
                              <div className="flex flex-col gap-1">
                                <div className="flex items-center gap-1">
                                  <Input
                                    aria-label={`Outcome ${i + 1}`}
                                    autoFocus={
                                      outcome.state.value === "" &&
                                      !outcome.state.meta.isTouched
                                    }
                                    value={outcome.state.value}
                                    placeholder="outcome_name"
                                    spellCheck={false}
                                    autoComplete="off"
                                    aria-invalid={invalid}
                                    onBlur={outcome.handleBlur}
                                    onChange={(e) =>
                                      outcome.handleChange(e.target.value)
                                    }
                                    className="h-8 font-mono text-xs"
                                  />
                                  <Button
                                    variant="ghost"
                                    size="icon-sm"
                                    aria-label={`Remove outcome ${outcome.state.value || i + 1}`}
                                    onClick={() => { onOutcomeRemove(i); field.removeValue(i) }}
                                  >
                                    <XIcon aria-hidden="true" />
                                  </Button>
                                </div>
                                {invalid ? (
                                  <FieldError
                                    errors={outcome.state.meta.errors}
                                  />
                                ) : null}
                              </div>
                            )
                          }}
                        </form.Field>
                      ))}
                      {attempted && !field.state.meta.isValid ? (
                        <FieldError
                          errors={field.state.meta.errors}
                        />
                      ) : null}
                      <Button
                        variant="ghost"
                        size="sm"
                        className="self-start pl-1.5 text-foreground-dim"
                        onClick={() => { onOutcomeAdd(); field.pushValue("") }}
                      >
                        <PlusIcon aria-hidden="true" /> Add outcome
                      </Button>
                    </Field>
                  )}
                </form.Field>
              </FieldGroup>
            </fieldset>
            {saveError ? (
              <div
                role="alert"
                className="mt-5 flex flex-col items-start gap-2"
              >
                <p className="text-sm text-destructive">
                  Could not save Task. Check project write access and try Save
                  again.
                </p>
                <p className="text-xs [overflow-wrap:anywhere] text-muted-foreground">
                  {saveError}
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  type="submit"
                  disabled={visualSaving || editing}
                >
                  Retry
                </Button>
              </div>
            ) : null}
          </form>
        ) : null}
      </div>
    </aside>
  )
}
