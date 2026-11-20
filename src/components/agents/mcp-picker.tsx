import { useState } from "react"
import { PlusIcon } from "lucide-react"
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
import type { McpSummary } from "@/lib/projects"
import { PickerStatus } from "@/components/picker-status"

/** MCP references use global/<id> for the machine-wide store. */
export function McpPicker({
  mcps,
  error,
  taken,
  onOpen,
  onPick,
}: {
  mcps?: McpSummary[]
  error?: string
  taken: string[]
  onOpen: () => void
  onPick: (name: string) => void
}) {
  const [open, setOpen] = useState(false)
  const options = mcps?.filter((m) => !taken.includes(m.name))
  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (next) onOpen()
      }}
    >
      <PopoverTrigger
        render={<Button variant="outline" size="sm" className="self-start" />}
      >
        <PlusIcon aria-hidden="true" />
        Add MCP
      </PopoverTrigger>
      <PopoverContent align="start" className="w-72 p-0">
        <Command>
          <CommandInput placeholder="Search MCPs…" />
          <CommandList>
            {options ? (
              <>
                <CommandEmpty>
                  {options.length ? "No MCPs match." : "No more MCPs."}
                </CommandEmpty>
                <CommandGroup>
                  {options.map((m) => (
                    <CommandItem
                      key={m.name}
                      value={m.name}
                      keywords={m.target ? [m.target] : undefined}
                      onSelect={() => {
                        onPick(m.name)
                        setOpen(false)
                      }}
                    >
                      <span className="flex min-w-0 flex-col">
                        <span className="truncate">
                          {m.name.startsWith("global/")
                            ? `${m.name.slice(7)} (Global)`
                            : m.name}
                        </span>
                        <span className="truncate font-mono text-xs text-muted-foreground">
                          {m.problem ?? `${m.kind} · ${m.target}`}
                        </span>
                      </span>
                    </CommandItem>
                  ))}
                </CommandGroup>
              </>
            ) : (
              <PickerStatus what="MCPs" error={error} onRetry={onOpen} />
            )}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
