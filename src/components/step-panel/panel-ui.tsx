import type { ReactNode } from "react"
import { CircleAlertIcon, PlusIcon, XIcon } from "lucide-react"
import { cn } from "cn"
import { Button } from "@/components/ui/button"
import { Field, FieldLabel, FieldTitle } from "@/components/ui/field"
import { taskRef } from "@/lib/step-edit"

export type Errors = { message?: string }[]

/** Field errors show once the field was left, or everywhere after Validate. */
export const shown = (meta: { isBlurred: boolean; errors: unknown[] }, all: boolean): Errors =>
  meta.isBlurred || all ? (meta.errors.filter(Boolean) as Errors) : []

export function ErrorText({ id, errors }: { id?: string; errors: Errors }) {
  const message = errors.find((e) => e.message)?.message
  if (!message) return null
  return (
    <p id={id} role="alert" className="flex items-center gap-1.5 text-xs text-destructive">
      <CircleAlertIcon aria-hidden="true" className="size-3.5 shrink-0" />
      {message}
    </p>
  )
}

/** Label column + control, for the short fields at the top of the panel. */
export function Row({
  label,
  htmlFor,
  labelId,
  labelClassName,
  errors,
  errorId,
  helper,
  children,
}: {
  label: string
  htmlFor?: string
  labelId?: string
  labelClassName?: string
  errors: Errors
  errorId: string
  helper?: ReactNode
  children: ReactNode
}) {
  return (
    <Field
      data-invalid={errors.length > 0}
      className="grid grid-cols-[76px_minmax(0,1fr)] items-center gap-x-2 gap-y-1.5"
    >
      {htmlFor ? (
        <FieldLabel htmlFor={htmlFor} className="text-foreground-dim">{label}</FieldLabel>
      ) : (
        <FieldTitle id={labelId} className={cn("text-foreground-dim", labelClassName)}>{label}</FieldTitle>
      )}
      {children}
      {errors.length > 0 ? (
        <div className="col-start-2">
          <ErrorText id={errorId} errors={errors} />
        </div>
      ) : helper ? (
        <p className="col-start-2 truncate text-xs text-muted-foreground">{helper}</p>
      ) : null}
    </Field>
  )
}

export function Section({
  label,
  source,
  action,
  children,
}: {
  label: string
  /** Named task the values come from; they are read-only here. */
  source?: string | null
  /** Header-row control, e.g. Edit instructions (agents inspector pattern). */
  action?: ReactNode
  children?: ReactNode
}) {
  return (
    <section aria-label={label} className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-sm font-medium text-foreground-dim">{label}</h3>
        {source ? (
          <span className="truncate text-xs text-muted-foreground">
            From {taskRef(source).global ? "global task" : "task"}{" "}
            <span className="font-mono">{taskRef(source).name}</span>
          </span>
        ) : (
          action
        )}
      </div>
      {children}
    </section>
  )
}

export const inlineAddClass = "self-start pl-1.5 text-foreground-dim"

export function InlineAdd({ label, onClick, className }: { label: string; onClick: () => void; className?: string }) {
  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={onClick}
      className={cn(inlineAddClass, className)}
    >
      <PlusIcon aria-hidden="true" /> {label}
    </Button>
  )
}

export function RemoveButton({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <Button variant="ghost" size="icon-xs" aria-label={label} title={label} onClick={onClick}>
      <XIcon aria-hidden="true" className="text-muted-foreground" />
    </Button>
  )
}

/** Bordered single-line row (artifact, output, outcome). */
export const rowFrame =
  "flex h-8 min-w-0 items-center gap-2 rounded-md border border-input bg-background pr-1 pl-2.5 text-sm"

/** Skill pills; read-only unless `onRemove` is given. */
export function SkillChips({ skills, onRemove }: { skills: string[]; onRemove?: (index: number) => void }) {
  if (!skills.length) return null
  return (
    <ul className="flex flex-wrap gap-1.5">
      {skills.map((skill, i) => (
        <li
          key={skill}
          className={cn(
            "flex h-7 items-center gap-1 rounded-md border border-input pr-0.5 pl-1.5 font-mono text-xs",
            !onRemove && "pr-1.5"
          )}
        >
          {skill}
          {onRemove && <RemoveButton label={`Remove ${skill}`} onClick={() => onRemove(i)} />}
        </li>
      ))}
    </ul>
  )
}
