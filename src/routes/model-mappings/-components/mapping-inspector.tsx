import { useStore } from "@tanstack/react-form"
import {
  PanelRightCloseIcon,
  Trash2Icon,
  TriangleAlertIcon,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import type { HarnessModels } from "../-hooks/use-harness-models"
import { HARNESSES } from "../-lib/model-mappings-list"
import {
  Field,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import type { MappingForm } from "../-hooks/use-mapping-form"
import { ModelPicker } from "./model-picker"

export function MappingInspector({
  original,
  form,
  dirty,
  busy,
  error,
  usedBy,
  harnessModels,
  onLoadModels,
  onDelete,
  onClose,
}: {
  /** null while creating a new alias */
  original: string | null
  form: MappingForm
  dirty: boolean
  busy: boolean
  error?: string
  usedBy: string[] | null
  harnessModels: Record<string, HarnessModels>
  onLoadModels: (harness: string) => void
  onDelete: () => Promise<void>
  onClose: () => void
}) {
  const draft = useStore(form.store, (s) => s.values)
  const canSubmit = useStore(form.store, (s) => s.canSubmit)
  const name = draft.alias || "this alias"

  return (
    <aside
      aria-label={original ? `Alias ${original}` : "New alias"}
      className="flex w-105 shrink-0 flex-col border-l bg-background"
    >
      <header className="flex h-16 shrink-0 items-center gap-2 border-b px-4">
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-base font-semibold">
            {original ?? "New alias"}
          </h2>
          <p className="truncate text-xs text-muted-foreground">
            {original
              ? `Global alias · ${usedBy?.length ? `used by ${usedBy.length} agent${usedBy.length === 1 ? "" : "s"}` : "not used in this project"}`
              : "Global alias"}
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
          form="mapping-form"
          disabled={!dirty || busy || !canSubmit}
        >
          {busy ? "Saving…" : original ? "Save" : "Create"}
        </Button>
        {original ? (
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={`Delete ${original}`}
            title="Delete alias"
            disabled={busy}
            onClick={() => {
              const n = usedBy?.length ?? 0
              if (
                window.confirm(
                  `Delete alias “${original}”?${n ? ` ${n} agent${n === 1 ? "" : "s"} in this project use it.` : ""}`
                )
              )
                void onDelete()
            }}
          >
            <Trash2Icon aria-hidden="true" />
          </Button>
        ) : null}
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Close panel"
          title="Close panel"
          onClick={onClose}
          disabled={busy}
        >
          <PanelRightCloseIcon aria-hidden="true" />
        </Button>
      </header>
      <form
        id="mapping-form"
        noValidate
        className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-4"
        onSubmit={(e) => {
          e.preventDefault()
          if (dirty && !busy) void form.handleSubmit()
        }}
      >
        <fieldset disabled={busy} className="min-w-0 disabled:opacity-60">
          <FieldGroup>
            <form.Field name="alias">
              {(field) => {
                const invalid =
                  (field.state.meta.isBlurred ||
                    form.state.submissionAttempts > 0) &&
                  !field.state.meta.isValid
                return (
                  <Field
                    data-invalid={invalid}
                    className="grid grid-cols-[6rem_1fr] items-start gap-x-3"
                  >
                    <FieldLabel
                      htmlFor="mapping-alias"
                      className="pt-2 text-sm"
                    >
                      Alias
                    </FieldLabel>
                    <div className="flex flex-col gap-1.5">
                      <Input
                        id="mapping-alias"
                        name={field.name}
                        value={field.state.value}
                        required
                        autoFocus={!original}
                        spellCheck={false}
                        autoComplete="off"
                        placeholder="e.g. smart"
                        aria-invalid={invalid}
                        aria-describedby={
                          invalid ? "mapping-alias-error" : undefined
                        }
                        onBlur={field.handleBlur}
                        onChange={(e) => {
                          field.handleChange(e.target.value)
                          if (field.state.meta.isBlurred) field.handleBlur()
                        }}
                        className="h-9 font-mono"
                      />
                      <div className="min-h-10">
                        {invalid ? (
                          <FieldError
                            id="mapping-alias-error"
                            errors={field.state.meta.errors}
                          />
                        ) : null}
                      </div>
                    </div>
                  </Field>
                )
              }}
            </form.Field>

            <section
              aria-labelledby="harness-models"
              className="flex flex-col gap-3 border-t pt-4"
            >
              <div className="flex flex-col gap-0.5">
                <h3 id="harness-models" className="text-sm font-medium">
                  Harness models
                </h3>
                <p className="text-xs text-muted-foreground">
                  Each alias needs a model for every harness that runs it.
                </p>
              </div>
              {HARNESSES.map((h) => {
                return (
                  <form.Field name={`models.${h.id}`} key={h.id}>
                    {(field) => (
                      <Field
                        key={h.id}
                        className="grid grid-cols-[6rem_1fr] items-start gap-x-3"
                        onBlur={field.handleBlur}
                      >
                        <FieldLabel
                          htmlFor={`model-${h.id}`}
                          className="pt-2 text-sm"
                        >
                          {h.name}
                        </FieldLabel>
                        <div className="flex min-w-0 flex-col gap-1.5">
                          <ModelPicker
                            id={`model-${h.id}`}
                            label={h.name}
                            value={field.state.value}
                            models={harnessModels[h.id]}
                            onOpen={() => onLoadModels(h.id)}
                            onChange={(model) => {
                              field.handleChange(model)
                              field.handleBlur()
                            }}
                            invalid={
                              (field.state.meta.isBlurred ||
                                form.state.submissionAttempts > 0) &&
                              !field.state.meta.isValid
                            }
                            errorId={
                              !field.state.meta.isValid
                                ? `model-${h.id}-error`
                                : undefined
                            }
                          />
                          {(field.state.meta.isBlurred ||
                            form.state.submissionAttempts > 0) &&
                          !field.state.meta.isValid ? (
                            <FieldError
                              id={`model-${h.id}-error`}
                              errors={field.state.meta.errors}
                            />
                          ) : null}
                          {!field.state.value ? (
                            <p className="flex gap-1.5 text-xs text-warning">
                              <TriangleAlertIcon
                                aria-hidden="true"
                                className="mt-px size-3.5 shrink-0"
                              />
                              Not mapped. Agents using {name} can’t run on{" "}
                              {h.name} until a model is selected.
                            </p>
                          ) : null}
                        </div>
                      </Field>
                    )}
                  </form.Field>
                )
              })}
            </section>
          </FieldGroup>
        </fieldset>

        {original ? (
          <div className="grid grid-cols-[6rem_1fr] gap-x-3 border-t pt-4 text-sm">
            <span>Used by</span>
            <span className="[overflow-wrap:anywhere] text-foreground-dim">
              {usedBy?.length ? usedBy.join(", ") : "—"}
            </span>
          </div>
        ) : null}

        {error ? (
          <p
            role="alert"
            className="text-sm [overflow-wrap:anywhere] text-destructive"
          >
            {error}
          </p>
        ) : null}
      </form>
    </aside>
  )
}
