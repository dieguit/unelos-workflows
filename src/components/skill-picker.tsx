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
import type { SkillSummary } from "@/lib/projects"
import { PickerStatus } from "@/components/picker-status"

/** "Add skill" popover listing skills not yet referenced. */
export function SkillPicker({
  skills,
  error,
  taken,
  onOpen,
  onPick,
}: {
  skills?: SkillSummary[]
  error?: string
  taken: string[]
  onOpen: () => void
  onPick: (name: string) => void
}) {
  const [open, setOpen] = useState(false)
  const options = skills?.filter((s) => !taken.includes(s.name))
  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (next) onOpen()
      }}
    >
      <PopoverTrigger
        render={
          <Button variant="outline" size="sm" className="self-start" />
        }
      >
        <PlusIcon aria-hidden="true" />
        Add skill
      </PopoverTrigger>
      <PopoverContent align="start" className="w-72 p-0">
        <Command>
          <CommandInput placeholder="Search skills…" />
          <CommandList>
            {options ? (
              <>
                <CommandEmpty>
                  {options.length ? "No skills match." : "No more skills."}
                </CommandEmpty>
                <CommandGroup>
                  {options.map((s) => (
                    <CommandItem
                      key={s.name}
                      value={s.name}
                      keywords={s.description ? [s.description] : undefined}
                      onSelect={() => {
                        onPick(s.name)
                        setOpen(false)
                      }}
                    >
                      <span className="flex min-w-0 flex-col">
                        <span className="truncate">{s.name}</span>
                        <span className="truncate font-mono text-xs text-muted-foreground">
                          skills/{s.name}
                        </span>
                      </span>
                    </CommandItem>
                  ))}
                </CommandGroup>
              </>
            ) : (
              <PickerStatus what="skills" error={error} onRetry={onOpen} />
            )}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
