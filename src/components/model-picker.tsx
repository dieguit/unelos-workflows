import { useRef, useState } from "react"
import {
  ChevronsUpDownIcon,
  CpuIcon,
  Loader2Icon,
  TriangleAlertIcon,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover"
import { cn } from "@/lib/utils"
import type { HarnessModels } from "@/hooks/use-harness-models"

export function ModelPicker({
  id,
  label,
  value,
  models,
  onOpen,
  onChange,
  invalid,
  errorId,
  neutral,
}: {
  id: string
  label: string
  value?: string
  models?: HarnessModels
  onOpen: () => void
  onChange: (model: string) => void
  invalid?: boolean
  errorId?: string
  /** Empty value is fine: no warning style. */
  neutral?: boolean
}) {
  const [open, setOpen] = useState(false)
  const listRef = useRef<HTMLDivElement>(null)
  const list = models?.models
  // Keep the saved model selectable even if the harness no longer lists it.
  const options =
    list && value && !list.includes(value) ? [value, ...list] : list
  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (next) onOpen()
      }}
    >
      <PopoverTrigger
        id={id}
        aria-label={`${label} model`}
        aria-invalid={invalid}
        aria-describedby={errorId}
        render={
          <Button
            variant="outline"
            className={cn(
              "h-9 w-full justify-start gap-2 px-2.5 font-normal",
              !value &&
                !neutral &&
                "border-warning bg-warning-muted/40 text-muted-foreground hover:bg-warning-muted/60"
            )}
          />
        }
      >
        {value ? (
          <CpuIcon aria-hidden="true" className="text-muted-foreground" />
        ) : neutral ? null : (
          <TriangleAlertIcon aria-hidden="true" className="text-warning" />
        )}
        <span
          className={cn(
            "min-w-0 flex-1 truncate text-left",
            value && "font-mono"
          )}
        >
          {value ?? "Select a model"}
        </span>
        <ChevronsUpDownIcon
          aria-hidden="true"
          className="text-muted-foreground"
        />
      </PopoverTrigger>
      <PopoverContent align="start" className="w-(--anchor-width) min-w-72 p-0">
        <Command>
          <CommandInput
            placeholder={`Search ${label} models…`}
            // cmdk keeps the old scroll offset while filtering; show the top match.
            onValueChange={() =>
              requestAnimationFrame(() => listRef.current?.scrollTo({ top: 0 }))
            }
          />
          <CommandList ref={listRef}>
            {options ? (
              <>
                <CommandEmpty>No models match.</CommandEmpty>
                <CommandGroup>
                  {options.map((m) => (
                    <CommandItem
                      key={m}
                      value={m}
                      data-checked={m === value}
                      onSelect={() => {
                        onChange(m)
                        setOpen(false)
                      }}
                      className="font-mono text-xs"
                    >
                      <span className="truncate">{m}</span>
                    </CommandItem>
                  ))}
                </CommandGroup>
              </>
            ) : models?.error ? (
              <div role="alert" className="flex flex-col items-start gap-2 p-3">
                <p className="text-sm [overflow-wrap:anywhere] text-destructive">
                  Couldn’t list {label} models: {models.error}
                </p>
                <Button variant="outline" size="sm" onClick={onOpen}>
                  Retry
                </Button>
              </div>
            ) : (
              <p
                role="status"
                className="flex items-center justify-center gap-2 py-6 text-sm text-muted-foreground"
              >
                <Loader2Icon
                  aria-hidden="true"
                  className="size-4 animate-spin"
                />
                Loading {label} models…
              </p>
            )}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
