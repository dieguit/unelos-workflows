import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import { useId, useState } from "react"
import { useForm, useStore } from "@tanstack/react-form"
import { CopyIcon, EllipsisIcon, GlobeIcon, Trash2Icon } from "lucide-react"
import { z } from "zod"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Field, FieldError, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"

export function ListActions({
  kind,
  name,
  nameSchema,
  duplicate,
  remove,
  beforeRemove,
  disabled,
  variant = "menu",
  makeGlobal,
  makeGlobalSkills,
  openGlobal,
}: {
  kind: string
  name: string
  nameSchema: z.ZodType<string, string>
  duplicate?: (name: string) => Promise<void>
  remove: () => Promise<void>
  beforeRemove?: () => Promise<boolean>
  disabled?: boolean
  /** "footer" renders inline buttons for an inspector panel instead of the dropdown. */
  variant?: "menu" | "footer"
  /** Moves a project item to global; omit on global pages. */
  makeGlobal?: () => Promise<void>
  /** Project skills that will be made global along with the item (tasks). */
  makeGlobalSkills?: () => Promise<string[]>
  openGlobal?: () => void
}) {
  const [mode, setMode] = useState<"duplicate" | "remove" | "global">()
  const [skills, setSkills] = useState<string[]>([])
  const id = useId()
  const [error, setError] = useState<string>()
  const [removing, setRemoving] = useState(false)
  const schema = z.object({ name: nameSchema })
  const form = useForm({
    defaultValues: { name: "" },
    validators: { onChange: schema, onBlur: schema, onSubmit: schema },
    onSubmit: async ({ value }) => {
      setError(undefined)
      try {
        await duplicate?.(value.name)
        setMode(undefined)
        toast.success(`${kind} duplicated`)
      } catch (err) {
        setError(String(err))
      }
    },
  })
  const saving = useStore(form.store, (s) => s.isSubmitting)
  const pending = saving || removing
  const visualPending = useDelayedFlag(pending)
  const open = async (next: "duplicate" | "remove" | "global") => {
    if (next !== "duplicate" && beforeRemove && !(await beforeRemove())) return
    setError(undefined)
    setSkills([])
    if (next === "global" && makeGlobalSkills)
      makeGlobalSkills().then(setSkills, (err) => setError(String(err)))
    form.reset({ name: "" })
    setMode(next)
  }
  const confirmRemove = async () => {
    setError(undefined)
    setRemoving(true)
    try {
      await remove()
      setMode(undefined)
      toast.success(`${kind} removed`)
    } catch (err) {
      setError(String(err))
    } finally {
      setRemoving(false)
    }
  }
  const confirmGlobal = async () => {
    setError(undefined)
    setRemoving(true)
    try {
      await makeGlobal?.()
      setMode(undefined)
      toast.success(`${kind} made global`, {
        action: openGlobal && { label: "Open", onClick: openGlobal },
      })
    } catch (err) {
      setError(String(err))
    } finally {
      setRemoving(false)
    }
  }
  return (
    <div
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => e.stopPropagation()}
      className={variant === "footer" ? "flex flex-wrap gap-2" : "flex justify-end"}
    >
      {variant === "footer" ? (
        <>
          {duplicate && (
            <Button variant="outline" size="sm" disabled={disabled} onClick={() => void open("duplicate")}>
              <CopyIcon aria-hidden="true" /> Duplicate
            </Button>
          )}
          {makeGlobal && (
            <Button variant="outline" size="sm" disabled={disabled} onClick={() => void open("global")}>
              <GlobeIcon aria-hidden="true" /> Make global
            </Button>
          )}
          <Button
            variant="outline"
            size="sm"
            disabled={disabled}
            className="ml-auto text-destructive hover:text-destructive"
            onClick={() => void open("remove")}
          >
            <Trash2Icon aria-hidden="true" /> Remove
          </Button>
        </>
      ) : (
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={`Actions for ${name}`}
              disabled={disabled}
            />
          }
        >
          <EllipsisIcon />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          {duplicate && <DropdownMenuItem onClick={() => void open("duplicate")}>
            <CopyIcon /> Duplicate
          </DropdownMenuItem>}
          {makeGlobal && <DropdownMenuItem onClick={() => void open("global")}>
            <GlobeIcon /> Make global
          </DropdownMenuItem>}
          <DropdownMenuItem
            variant="destructive"
            onClick={() => void open("remove")}
          >
            <Trash2Icon /> Remove
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      )}
      <Dialog
        open={!!mode}
        onOpenChange={(open) => {
          if (!open && !pending) setMode(undefined)
        }}
      >
        <DialogContent inert={pending} showCloseButton={!visualPending}>
          <DialogTitle>
            {mode === "duplicate"
              ? `Duplicate ${kind}`
              : mode === "global"
                ? `Make ${kind} global?`
                : `Remove ${kind}?`}
          </DialogTitle>
          <DialogDescription>
            {mode === "duplicate"
              ? `Choose a name for the copy of ${name}.`
              : mode === "global"
                ? `${name} moves to ~/.config/unelos and becomes available to all projects. References in this project are updated to global/${name}.`
                : `Remove ${name}? This cannot be undone.`}
          </DialogDescription>
          {mode === "global" && skills.length > 0 && (
            <div className="text-sm">
              <p>These project skills are also made global:</p>
              <ul className="mt-1 list-disc pl-5 font-mono text-xs">
                {skills.map((sk) => (
                  <li key={sk}>{sk}</li>
                ))}
              </ul>
            </div>
          )}
          {mode === "duplicate" && (
            <form
              id={`${id}-form`}
              noValidate
              onSubmit={(e) => {
                e.preventDefault()
                if (!pending) void form.handleSubmit()
              }}
            >
              <form.Field name="name">
                {(field) => {
                  const invalid =
                    (field.state.meta.isBlurred ||
                      form.state.submissionAttempts > 0) &&
                    !field.state.meta.isValid
                  return (
                    <Field data-invalid={invalid}>
                      <FieldLabel htmlFor={`${id}-name`}>Name</FieldLabel>
                      <Input
                        id={`${id}-name`}
                        name={field.name}
                        required
                        autoFocus
                        disabled={visualPending}
                        value={field.state.value}
                        onBlur={field.handleBlur}
                        onChange={(e) => field.handleChange(e.target.value)}
                        aria-invalid={invalid}
                        aria-describedby={invalid ? `${id}-error` : undefined}
                      />
                      <FieldError
                        id={`${id}-error`}
                        errors={invalid ? field.state.meta.errors : []}
                      />
                    </Field>
                  )
                }}
              </form.Field>
            </form>
          )}
          {error && (
            <p
              role="alert"
              className="text-sm [overflow-wrap:anywhere] text-destructive"
            >
              {error}
            </p>
          )}
          <DialogFooter>
            <Button
              variant="outline"
              disabled={visualPending}
              onClick={() => setMode(undefined)}
            >
              Cancel
            </Button>
            {mode === "duplicate" ? (
              <Button type="submit" form={`${id}-form`} disabled={visualPending}>
                {visualPending && saving ? "Duplicating…" : "Duplicate"}
              </Button>
            ) : mode === "global" ? (
              <Button disabled={visualPending} onClick={() => void confirmGlobal()}>
                {visualPending && removing ? "Moving…" : "Make global"}
              </Button>
            ) : (
              <Button
                variant="destructive"
                disabled={visualPending}
                onClick={() => void confirmRemove()}
              >
                {visualPending && removing ? "Removing…" : "Remove"}
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
