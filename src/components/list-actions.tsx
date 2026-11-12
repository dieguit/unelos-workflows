import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import { useId, useState } from "react"
import { useForm, useStore } from "@tanstack/react-form"
import { CopyIcon, EllipsisIcon, Trash2Icon } from "lucide-react"
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
}: {
  kind: string
  name: string
  nameSchema: z.ZodType<string, string>
  duplicate?: (name: string) => Promise<void>
  remove: () => Promise<void>
  beforeRemove?: () => Promise<boolean>
  disabled?: boolean
}) {
  const [mode, setMode] = useState<"duplicate" | "remove">()
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
  const open = async (next: "duplicate" | "remove") => {
    if (next === "remove" && beforeRemove && !(await beforeRemove())) return
    setError(undefined)
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
  return (
    <div
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => e.stopPropagation()}
      className="flex justify-end"
    >
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
          <DropdownMenuItem
            variant="destructive"
            onClick={() => void open("remove")}
          >
            <Trash2Icon /> Remove
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <Dialog
        open={!!mode}
        onOpenChange={(open) => {
          if (!open && !pending) setMode(undefined)
        }}
      >
        <DialogContent inert={pending} showCloseButton={!visualPending}>
          <DialogTitle>
            {mode === "duplicate" ? `Duplicate ${kind}` : `Remove ${kind}?`}
          </DialogTitle>
          <DialogDescription>
            {mode === "duplicate"
              ? `Choose a name for the copy of ${name}.`
              : `Remove ${name}? This cannot be undone.`}
          </DialogDescription>
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
