import { useState } from "react"
import { ChevronsUpDownIcon, FlagIcon, SquareDashedIcon } from "lucide-react"
import { cn } from "cn"
import { Button } from "@/components/ui/button"
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import type { WorkflowStep } from "@/lib/projects"
import { humanize } from "@/lib/step-edit"
import { stepIconColors, stepIcons } from "@/components/flow/step-icon"

/** Step combobox. `start` lists steps a run can begin at; `destination` adds End steps and leaves out `exclude`. */
export function StepPicker({
  id,
  variant,
  purpose = "start",
  exclude,
  steps,
  value,
  onChange,
  onBlur,
  disabled,
  invalid,
  describedBy,
}: {
  id: string
  variant: "compact" | "field"
  purpose?: "start" | "destination"
  exclude?: string
  steps: WorkflowStep[]
  value: string
  onChange: (value: string) => void
  onBlur: () => void
  disabled?: boolean
  invalid?: boolean
  describedBy?: string
}) {
  const [open, setOpen] = useState(false)
  // End steps cannot start a run; keep a current one visible so validation can flag it.
  const options = steps.filter((step) =>
    purpose === "start" ? step.kind !== "end" || step.id === value : step.id !== exclude
  )
  const selected = options.find((step) => step.id === value)
  const missing = value && !selected ? value : null
  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) onBlur()
      }}
    >
      <PopoverTrigger
        id={id}
        disabled={disabled}
        aria-invalid={invalid}
        aria-describedby={describedBy}
        render={
          <Button
            variant="outline"
            size={variant === "compact" ? "sm" : "default"}
            className={cn(
              "justify-start font-normal",
              variant === "field" && "h-9 w-full",
              invalid && "border-destructive"
            )}
          />
        }
      >
        {variant === "field" &&
          (purpose === "start" ? (
            <FlagIcon aria-hidden="true" className={selected ? "text-success" : "text-muted-foreground"} />
          ) : selected ? (
            <SelectedIcon kind={selected.kind} />
          ) : (
            <SquareDashedIcon aria-hidden="true" className="text-muted-foreground" />
          ))}
        <span className={cn("min-w-0 truncate text-left", variant === "field" && "flex-1")}>
          {selected ? (
            <>
              {humanize(selected.id)}
              {variant === "field" && (
                <span className="ml-2 font-mono text-xs text-muted-foreground">{selected.id}</span>
              )}
            </>
          ) : missing ? (
            <span className="text-destructive">Missing step: {missing}</span>
          ) : (
            <span className="text-muted-foreground">{purpose === "start" ? "Choose a step" : "Select step"}</span>
          )}
        </span>
        <ChevronsUpDownIcon aria-hidden="true" className="text-muted-foreground" />
      </PopoverTrigger>
      <PopoverContent align="start" className="w-(--anchor-width) min-w-64 p-0">
        <Command>
          <CommandInput placeholder="Search steps…" />
          <CommandList>
            <CommandEmpty>No steps match.</CommandEmpty>
            <CommandGroup>
              {missing && (
                <CommandItem value={`missing ${missing}`} disabled data-checked>
                  <span className="text-destructive">Missing step: {missing}</span>
                </CommandItem>
              )}
              {options.map((step) => {
                const Icon = stepIcons[step.kind]
                return (
                  <CommandItem
                    key={step.id}
                    value={`${humanize(step.id)} ${step.id}`}
                    data-checked={step.id === value}
                    onSelect={() => {
                      onChange(step.id)
                      setOpen(false)
                      onBlur()
                    }}
                  >
                    <Icon aria-hidden="true" className={stepIconColors[step.kind]} />
                    <span className="truncate">{humanize(step.id)}</span>
                    <span className="truncate font-mono text-xs text-muted-foreground">{step.id}</span>
                  </CommandItem>
                )
              })}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}

function SelectedIcon({ kind }: { kind: WorkflowStep["kind"] }) {
  const Icon = stepIcons[kind]
  return <Icon aria-hidden="true" className={stepIconColors[kind]} />
}
