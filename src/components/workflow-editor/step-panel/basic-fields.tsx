import { useCallback, useRef, useState } from "react"
import { BotIcon, ChevronsUpDownIcon, GitBranchIcon, TerminalIcon } from "lucide-react"
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
import { Input } from "@/components/ui/input"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { PickerStatus } from "@/components/picker-status"
import { useLazy } from "@/hooks/use-lazy"
import { listAgents, type WorkflowStep } from "@/lib/projects"
import type { WorkflowForm } from "../../-hooks/use-workflow-form"
import { humanize, renameStep, slugify, stepNameError } from "@/lib/step-edit"
import { Row, shown } from "@/components/step-panel/panel-ui"

type Props = { form: WorkflowForm; index: number; showErrors: boolean }

/** Display name; the id is its slug and renames every reference as you type. */
export function NameField({ form, index, showErrors, step, nameDraft, onNameDraft }: Props & { step: WorkflowStep; nameDraft?: { text: string; error: string }; onNameDraft: (id: string, draft?: { text: string; error: string }) => void }) {
  const [text, setText] = useState(() => nameDraft?.text ?? humanize(step.id))
  // A new step's placeholder name is selected so typing replaces it.
  const fresh = useRef(step.originalId === null)
  return (
    <form.Field name={`steps[${index}].id`}>
      {(field) => {
        const errors = nameDraft
          ? field.state.meta.isBlurred || showErrors ? [{ message: nameDraft.error }] : []
          : shown(field.state.meta, showErrors)
        return (
          <Row
            label="Name"
            htmlFor="step-name"
            errors={errors}
            errorId="step-name-error"
            helper={
              <>
                id <span className="font-mono">{step.id}</span>
              </>
            }
          >
            <Input
              id="step-name"
              value={text}
              autoFocus={step.originalId === null}
              spellCheck={false}
              autoComplete="off"
              aria-invalid={errors.length > 0}
              aria-describedby={errors.length ? "step-name-error" : undefined}
              onFocus={(e) => {
                if (fresh.current) e.currentTarget.select()
                fresh.current = false
              }}
              onBlur={field.handleBlur}
              onChange={(e) => {
                setText(e.target.value)
                const slug = slugify(e.target.value)
                const { steps: current, start } = form.state.values
                const from = current[index].id
                const error = stepNameError(e.target.value, current, index)
                onNameDraft(from, error ? { text: e.target.value, error } : undefined)
                if (error) return
                if (slug === from) return
                const renamed = renameStep(current, start, from, slug)
                form.setFieldValue("steps", renamed.steps)
                form.setFieldValue("start", renamed.start)
              }}
            />
          </Row>
        )
      }}
    </form.Field>
  )
}

export function useAgents(root: string) {
  const fetchAgents = useCallback(() => listAgents(root), [root])
  return useLazy(fetchAgents)
}

/** Agent of the step, or of one of its parallel tasks when `worker` is set. */
export function AgentField({ agents, form, index, worker, showErrors }: Props & { agents: ReturnType<typeof useAgents>; worker?: number }) {
  const [open, setOpen] = useState(false)
  return (
    <form.Field name={worker === undefined ? `steps[${index}].agent` : `steps[${index}].workers[${worker}].agent`}>
      {(field) => {
        const errors = shown(field.state.meta, showErrors)
        const value = field.state.value
        return (
          <Row label="Agent" htmlFor="step-agent" errors={errors} errorId="step-agent-error">
            <Popover
              open={open}
              onOpenChange={(next) => {
                setOpen(next)
                if (next) agents.load()
                else field.handleBlur()
              }}
            >
              <PopoverTrigger
                id="step-agent"
                aria-invalid={errors.length > 0}
                aria-describedby={errors.length ? "step-agent-error" : undefined}
                render={<Button variant="outline" className={value ? "h-auto w-full justify-start py-1.5 font-normal" : "w-full justify-start font-normal"} />}
              >
                <BotIcon aria-hidden="true" className="text-muted-foreground" />
                {value ? (
                  <span className="flex min-w-0 flex-1 flex-col text-left">
                    <span className="truncate">{humanize(value)}</span>
                    <span className="truncate font-mono text-xs text-muted-foreground">agents/{value}</span>
                  </span>
                ) : (
                  <span className="flex-1 text-left text-muted-foreground">Select agent</span>
                )}
                <ChevronsUpDownIcon aria-hidden="true" className="text-muted-foreground" />
              </PopoverTrigger>
              <PopoverContent align="start" className="w-(--anchor-width) min-w-64 p-0">
                <Command>
                  <CommandInput placeholder="Search agents…" />
                  <CommandList>
                    {agents.data ? (
                      <>
                        <CommandEmpty>{agents.data.length ? "No agents match." : "No project agents."}</CommandEmpty>
                        <CommandGroup>
                          {agents.data.map((agent) => (
                            <CommandItem
                              key={agent.name}
                              value={agent.name}
                              data-checked={agent.name === value}
                              onSelect={() => {
                                field.handleChange(agent.name)
                                setOpen(false)
                                field.handleBlur()
                              }}
                            >
                              <BotIcon aria-hidden="true" className="text-muted-foreground" />
                              <span className="flex min-w-0 flex-1 flex-col">
                                <span className="truncate">{humanize(agent.name)}</span>
                                <span className="truncate font-mono text-xs text-muted-foreground">
                                  agents/{agent.name}
                                </span>
                              </span>
                            </CommandItem>
                          ))}
                        </CommandGroup>
                      </>
                    ) : (
                      <PickerStatus what="agents" error={agents.error} onRetry={agents.load} />
                    )}
                  </CommandList>
                </Command>
              </PopoverContent>
            </Popover>
          </Row>
        )
      }}
    </form.Field>
  )
}

const executions = [
  { value: "main", label: "Main session", icon: TerminalIcon },
  { value: "subagent", label: "Subagent", icon: GitBranchIcon },
] as const

export function ExecutionField({ form, index, showErrors }: Props) {
  return (
    <form.Field name={`steps[${index}].execution`}>
      {(field) => {
        const errors = shown(field.state.meta, showErrors)
        return (
          <Row label="Execution" labelId="step-execution-label" errors={errors} errorId="step-execution-error">
            <div
              role="group"
              aria-labelledby="step-execution-label"
              aria-describedby={errors.length ? "step-execution-error" : undefined}
              className={cn(
                "grid h-8 grid-cols-2 gap-0.5 rounded-md bg-muted p-0.5",
                errors.length > 0 && "ring-1 ring-destructive"
              )}
            >
              {executions.map(({ value, label, icon: Icon }) => {
                const on = field.state.value === value
                return (
                  <button
                    key={value}
                    type="button"
                    aria-pressed={on}
                    onClick={() => {
                      field.handleChange(value)
                      field.handleBlur()
                    }}
                    className={cn(
                      "flex items-center justify-center gap-1.5 rounded-sm border border-transparent text-sm text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none",
                      on && "border-border-strong bg-popover font-medium text-foreground"
                    )}
                  >
                    <Icon aria-hidden="true" className={cn("size-3.5", on && "text-primary")} />
                    {label}
                  </button>
                )
              })}
            </div>
          </Row>
        )
      }}
    </form.Field>
  )
}
