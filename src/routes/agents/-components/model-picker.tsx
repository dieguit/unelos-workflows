import { useEffect, useState } from "react"
import { ChevronsUpDownIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { agentModelOptions, type AgentModel } from "@/lib/projects"
import { useLazy } from "@/hooks/use-lazy"
import { PickerStatus } from "@/components/picker-status"

export function AgentModelPicker({ value, onChange }: {
  value: AgentModel | null
  onChange: (value: AgentModel | null) => void
}) {
  const [open, setOpen] = useState(false)
  const options = useLazy(agentModelOptions)
  const { load } = options
  useEffect(() => load(), [load])
  const pick = (model: AgentModel | null) => {
    onChange(model)
    setOpen(false)
  }
  const harnessName = typeof value === "object" && value
    ? options.data?.harnesses.find((h) => h.id === value.harness)?.name ?? value.harness
    : null
  return (
    <Popover open={open} onOpenChange={(next) => {
      setOpen(next)
      if (next) options.load()
    }}>
      <PopoverTrigger id="agent-model" aria-label="Agent model" render={<Button variant="outline" className="h-auto min-h-9 w-full justify-start font-normal" />}>
        <span className="min-w-0 flex-1 truncate text-left">
          {typeof value === "string" ? value : value?.model ?? "Not configured"}
          {harnessName ? <span className="ml-2 text-xs text-muted-foreground">{harnessName}</span> : null}
        </span>
        <ChevronsUpDownIcon aria-hidden="true" className="text-muted-foreground" />
      </PopoverTrigger>
      <PopoverContent align="start" className="w-(--anchor-width) min-w-72 p-0">
        <Command>
          <CommandInput placeholder="Search aliases and models…" />
          <CommandList>
            <CommandGroup>
              <CommandItem value="Not configured" data-checked={!value} onSelect={() => pick(null)}>Not configured</CommandItem>
            </CommandGroup>
            {options.data ? <>
              <CommandEmpty>No models match.</CommandEmpty>
              <CommandGroup heading="Aliases">
                {Object.keys(options.data.mappings).map((alias) => <CommandItem key={alias} value={`alias ${alias}`} data-checked={value === alias} onSelect={() => pick(alias)}>{alias}</CommandItem>)}
              </CommandGroup>
              {options.data.harnesses.map((h) => h.error ? (
                <div key={h.id} className="px-3 py-2 text-xs text-muted-foreground">
                  <p className="mb-1 font-medium">{h.name}</p>
                  <p className="[overflow-wrap:anywhere]">{h.error}</p>
                </div>
              ) : (
                <CommandGroup key={h.id} heading={h.name}>
                  {h.models.map(({ id: model }) => <CommandItem key={model} value={`${h.name} ${model}`} data-checked={typeof value === "object" && value?.harness === h.id && value.model === model} onSelect={() => pick({ harness: h.id, model })}>
                    <span className="truncate font-mono text-xs">{model}</span>
                  </CommandItem>)}
                </CommandGroup>
              ))}
              {!Object.keys(options.data.mappings).length && !options.data.harnesses.some((h) => !h.error && h.models.length) ? <p className="p-3 text-sm text-muted-foreground">No configured models</p> : null}
            </> : <PickerStatus what="models" error={options.error} onRetry={options.load} />}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
