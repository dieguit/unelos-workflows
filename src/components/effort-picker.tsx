import { useState } from "react"
import { ChevronsUpDownIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Command, CommandGroup, CommandItem, CommandList } from "@/components/ui/command"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"

export function EffortPicker({ id, label, value, choices, leading, onChange, disabled }: {
  id: string
  label: string
  value: string | undefined
  choices: { value: string; stale: boolean }[]
  leading: { value: string | undefined; label: string }[]
  onChange: (value: string | undefined) => void
  disabled?: boolean
}) {
  const [open, setOpen] = useState(false)
  const selected = leading.find((option) => option.value === value)?.label ?? value
  const pick = (next: string | undefined) => {
    onChange(next)
    setOpen(false)
  }
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger id={id} aria-label={label} disabled={disabled}
        render={<Button variant="outline" className="h-9 w-full justify-start font-normal" />}>
        <span className="min-w-0 flex-1 truncate text-left">{selected}</span>
        <ChevronsUpDownIcon aria-hidden="true" className="text-muted-foreground" />
      </PopoverTrigger>
      <PopoverContent align="start" className="w-(--anchor-width) min-w-56 p-0">
        <Command>
          <CommandList>
            <CommandGroup>
              {leading.map((option, index) => (
                <CommandItem key={index} value={`leading-${index}`} data-checked={value === option.value} onSelect={() => pick(option.value)}>
                  {option.label}
                </CommandItem>
              ))}
              {choices.map((option) => (
                <CommandItem key={option.value} value={option.value} data-checked={value === option.value} onSelect={() => pick(option.value)}>
                  <span>{option.value}</span>
                  {option.stale ? <span className="text-xs text-warning">Not offered by this model</span> : null}
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
