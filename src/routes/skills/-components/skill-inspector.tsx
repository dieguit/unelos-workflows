import { BusyContent } from "@/components/busy-content"
import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import type { ReactNode } from "react"
import { PanelRightCloseIcon, PencilIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  Field,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import type { SkillForm } from "../-hooks/use-skill-form"
import type { SkillDocument } from "@/lib/projects"

function Shell({
  title,
  action,
  dirty,
  saving,
  onClose,
  children,
}: {
  title: string
  action?: ReactNode
  dirty?: boolean
  saving?: boolean
  onClose: () => void
  children: ReactNode
}) {
  const visualSaving = useDelayedFlag(!!saving)
  return (
    <aside inert={saving}
      aria-label={`Skill ${title}`}
      className="flex w-[380px] max-w-full shrink-0 flex-col border-l bg-background"
    >
      <header className="flex h-16 shrink-0 items-center gap-2 px-4">
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-base font-semibold">{title}</h2>
          {dirty ? (
            <p role="status" className="mt-1 text-xs text-warning">
              Unsaved changes
            </p>
          ) : null}
        </div>
        {action}
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Close panel"
          title="Close panel"
          onClick={onClose}
          disabled={visualSaving}
        >
          <PanelRightCloseIcon aria-hidden="true" />
        </Button>
      </header>
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto p-4">
        {children}
      </div>
    </aside>
  )
}

export function SkillInspector({
  id,
  doc,
  error,
  onClose,
  onEdit,
  form,
  dirty,
  saving,
  saveError,
  editing,
}: {
  id: string
  doc?: SkillDocument
  error?: string
  onClose: () => void
  onEdit: () => void
  form: SkillForm
  dirty: boolean
  saving: boolean
  saveError?: string
  editing: boolean
}) {
  const visualSaving = useDelayedFlag(saving)
  if (!doc)
    return (
      <Shell title={id} onClose={onClose}>
        {error ? (
          <p
            role="alert"
            className="text-sm [overflow-wrap:anywhere] text-destructive"
          >
            Couldn’t load skill: {error}
          </p>
        ) : (
          <p role="status" className="text-sm text-muted-foreground">
            Loading skill…
          </p>
        )}
      </Shell>
    )
  return (
    <Shell
      title={doc.name}
      onClose={onClose}
      dirty={dirty}
      saving={saving}
      action={
        <Button
          size="sm"
          form="skill-form"
          type="submit"
          disabled={!dirty || visualSaving || editing}
        >
          <BusyContent busy={visualSaving}>Save</BusyContent>
        </Button>
      }
    >
      <form
        id="skill-form"
        noValidate
        className="flex flex-col gap-5"
        onSubmit={(e) => {
          e.preventDefault()
          if (dirty && !saving && !editing) void form.handleSubmit()
        }}
      >
        <fieldset
          disabled={visualSaving || editing}
          className="flex min-w-0 flex-col gap-5 disabled:opacity-60"
        >
          <FieldGroup className="gap-4">
            <form.Field name="name">
              {(field) => {
                const invalid =
                  (field.state.meta.isBlurred ||
                    form.state.submissionAttempts > 0) &&
                  !field.state.meta.isValid
                return (
                  <Field data-invalid={invalid} className="grid grid-cols-[76px_minmax(0,1fr)] items-center gap-x-2 [&>label]:text-foreground-dim [&>[data-slot=field-error]]:col-start-2">
                    <FieldLabel htmlFor="skill-name">Name</FieldLabel>
                    <Input
                      id="skill-name"
                      name={field.name}
                      value={field.state.value}
                      required
                      spellCheck={false}
                      aria-invalid={invalid}
                      aria-describedby={
                        invalid ? "skill-name-error" : undefined
                      }
                      onBlur={field.handleBlur}
                      onChange={(e) => {
                        field.handleChange(e.target.value)
                        if (field.state.meta.isBlurred) field.handleBlur()
                      }}
                      className="h-9 font-normal"
                    />
                    {invalid ? (
                      <FieldError
                        id="skill-name-error"
                        errors={field.state.meta.errors}
                      />
                    ) : null}
                  </Field>
                )
              }}
            </form.Field>
            <form.Field name="description">
              {(field) => (
                <Field className="grid grid-cols-[76px_minmax(0,1fr)] items-center gap-x-2 [&>label]:text-foreground-dim">
                  <FieldLabel htmlFor="skill-description">
                    Description
                  </FieldLabel>
                  <Input
                    id="skill-description"
                    name={field.name}
                    value={field.state.value}
                    onBlur={field.handleBlur}
                    onChange={(e) => {
                      field.handleChange(e.target.value)
                      if (field.state.meta.isBlurred) field.handleBlur()
                    }}
                    className="h-9 font-normal"
                  />
                </Field>
              )}
            </form.Field>
          </FieldGroup>
        </fieldset>
        {saveError ? (
          <div role="alert" className="flex flex-col items-start gap-2 text-sm text-destructive">
            <p className="[overflow-wrap:anywhere]">Couldn’t save: {saveError}</p>
            <Button type="submit" variant="outline" size="sm" disabled={visualSaving || editing}>Retry</Button>
          </div>
        ) : null}
      </form>
      <form.Field name="body">
        {(field) => (
          <Field aria-labelledby="skill-body">
            <div className="flex items-center justify-between">
              <h3 id="skill-body" className="text-sm font-medium">
                SKILL.md
              </h3>
              <Button
                variant="outline"
                size="sm"
                onClick={onEdit}
                disabled={visualSaving || editing}
              >
                <PencilIcon aria-hidden="true" />
                Edit
              </Button>
            </div>
            <pre className="line-clamp-5 rounded-lg border bg-card p-3 font-mono text-xs leading-relaxed [overflow-wrap:anywhere] whitespace-pre-wrap text-foreground-dim">
              {field.state.value.trim() || "Empty"}
            </pre>
          </Field>
        )}
      </form.Field>
    </Shell>
  )
}
