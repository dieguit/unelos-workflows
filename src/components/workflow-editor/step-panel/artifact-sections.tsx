import { useState } from "react"
import { useStore } from "@tanstack/react-form"
import { FileInputIcon, FileOutputIcon, PlusIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import { Input } from "@/components/ui/input"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import type { WorkflowStep } from "@/lib/projects"
import type { WorkflowForm } from "@/hooks/use-workflow-form"
import { humanize, producers } from "@/lib/step-edit"
import { ErrorText, InlineAdd, inlineAddClass, RemoveButton, rowFrame, Section } from "@/components/step-panel/panel-ui"

export function inputSources(steps: WorkflowStep[], index: number, declared: string[]) {
  const sources = new Map<string, string | null>(declared.map((name) => [name, null]))
  for (const { artifact, step } of producers(steps, index)) sources.set(artifact, step)
  return sources
}

export function InputsSection({ form, index, steps }: { form: WorkflowForm; index: number; steps: WorkflowStep[] }) {
  const [open, setOpen] = useState(false)
  const declared = useStore(form.store, (s) => s.values.inputs)
  const producerOf = inputSources(steps, index, declared)
  return (
    <form.Field name={`steps[${index}].inputs`} mode="array">
      {(field) => {
        const bound = field.state.value
        const options = [...producerOf].filter(([artifact]) => !bound.includes(artifact))
        return (
          <Section label="Inputs">
            {bound.map((artifact, i) => {
              const from = producerOf.get(artifact)
              return (
                <div key={artifact} className={rowFrame}>
                  <FileInputIcon aria-hidden="true" className="size-3.5 shrink-0 text-muted-foreground" />
                  <span className="min-w-0 flex-1 truncate font-mono text-xs">{artifact}</span>
                  {from && (
                    <span className="flex h-5 shrink-0 items-center gap-1 rounded-sm bg-muted px-1.5 text-xs text-muted-foreground">
                      from <span className="font-medium text-foreground-dim">{humanize(from)}</span>
                    </span>
                  )}
                  <RemoveButton label={`Remove input ${artifact}`} onClick={() => field.removeValue(i)} />
                </div>
              )
            })}
            <Popover open={open} onOpenChange={setOpen}>
              <PopoverTrigger render={<Button variant="ghost" size="sm" className={inlineAddClass} />}>
                <PlusIcon aria-hidden="true" /> Bind input
              </PopoverTrigger>
              <PopoverContent align="start" className="w-80 p-0">
                <Command>
                  <CommandInput placeholder="Search available inputs…" />
                  <CommandList>
                    <CommandEmpty>
                      {options.length ? "No inputs match." : "No available inputs left to bind."}
                    </CommandEmpty>
                    <CommandGroup>
                      {options.map(([artifact, step]) => (
                        <CommandItem
                          key={artifact}
                          value={`${artifact} ${step ?? "workflow input"}`}
                          onSelect={() => {
                            field.pushValue(artifact)
                            setOpen(false)
                          }}
                        >
                          <FileInputIcon aria-hidden="true" className="text-muted-foreground" />
                          <span className="truncate font-mono text-xs">{artifact}</span>
                          <span className="ml-auto shrink-0 text-xs text-muted-foreground">{step ? `from ${humanize(step)}` : "Workflow input"}</span>
                        </CommandItem>
                      ))}
                    </CommandGroup>
                  </CommandList>
                </Command>
              </PopoverContent>
            </Popover>
          </Section>
        )
      }}
    </form.Field>
  )
}

export function OutputsSection({ form, index }: { form: WorkflowForm; index: number }) {
  const [draft, setDraft] = useState<string | null>(null)
  return (
    <form.Field name={`steps[${index}].outputs`} mode="array">
      {(field) => {
        const outputs = field.state.value
        const name = draft?.trim() ?? ""
        const duplicate = outputs.includes(name)
        const commit = () => {
          if (name && !duplicate) field.pushValue(name)
          if (!duplicate) setDraft(null)
        }
        return (
          <Section label="Outputs">
            {outputs.map((output, i) => (
              <div key={output} className={rowFrame}>
                <FileOutputIcon aria-hidden="true" className="size-3.5 shrink-0 text-muted-foreground" />
                <span className="min-w-0 flex-1 truncate font-mono text-xs">{output}</span>
                <RemoveButton label={`Remove output ${output}`} onClick={() => field.removeValue(i)} />
              </div>
            ))}
            {draft === null ? (
              <InlineAdd label="Add output" onClick={() => setDraft("")} />
            ) : (
              <>
                <Input
                  autoFocus
                  aria-label="New output file"
                  placeholder="report.md"
                  spellCheck={false}
                  value={draft}
                  aria-invalid={duplicate}
                  aria-describedby={duplicate ? "step-output-error" : undefined}
                  className="h-8 font-mono text-xs"
                  onChange={(e) => setDraft(e.target.value)}
                  onBlur={() => (duplicate ? setDraft(null) : commit())}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault()
                      commit()
                    } else if (e.key === "Escape") {
                      e.preventDefault()
                      setDraft(null)
                    }
                  }}
                />
                {duplicate && <ErrorText id="step-output-error" errors={[{ message: `“${name}” is already an output.` }]} />}
              </>
            )}
          </Section>
        )
      }}
    </form.Field>
  )
}

export function WorkflowInputsSection({ form }: { form: WorkflowForm }) {
  const [draft, setDraft] = useState<string | null>(null)
  return <form.Field name="inputs" mode="array">
    {(field) => {
      const name = draft?.trim() ?? ""
      const duplicate = field.state.value.includes(name)
      const commit = () => {
        if (name && !duplicate) field.pushValue(name)
        if (!duplicate) setDraft(null)
      }
      return <Section label="Workflow inputs">
        {field.state.value.map((input, i) => <div key={input} className={rowFrame}>
          <span className="min-w-0 flex-1 truncate font-mono text-xs">{input}</span>
          <RemoveButton label={`Remove workflow input ${input}`} onClick={() => field.removeValue(i)} />
        </div>)}
        {draft === null ? <InlineAdd label="Add workflow input" onClick={() => setDraft("")} /> : <>
          <Input autoFocus aria-label="New workflow input" placeholder="brief.md" value={draft} aria-invalid={duplicate} onChange={(e) => setDraft(e.target.value)} onBlur={() => duplicate ? setDraft(null) : commit()} onKeyDown={(e) => {
            if (e.key === "Enter") { e.preventDefault(); commit() }
            if (e.key === "Escape") { e.preventDefault(); setDraft(null) }
          }} />
          {duplicate && <ErrorText errors={[{ message: `“${name}” is already an input.` }]} />}
        </>}
      </Section>
    }}
  </form.Field>
}
