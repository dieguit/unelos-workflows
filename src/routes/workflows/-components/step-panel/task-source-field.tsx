import { useCallback, useState } from "react"
import { Link } from "@tanstack/react-router"
import {
  ChevronsUpDownIcon,
  ExternalLinkIcon,
  GlobeIcon,
  ListTodoIcon,
  PencilLineIcon,
} from "lucide-react"
import { cn } from "cn"
import { Button, buttonVariants } from "@/components/ui/button"
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
import { PickerStatus } from "@/components/picker-status"
import { useDiscardChanges } from "@/hooks/use-discard-changes"
import { useLazy } from "@/hooks/use-lazy"
import {
  getGlobalTask,
  getTask,
  listGlobalTasks,
  listTasks,
  type StepTask,
} from "@/lib/projects"
import type { WorkflowForm } from "../../-hooks/use-workflow-form"
import { humanize, taskRef } from "../../-lib/step-edit"
import { Row } from "./panel-ui"

const segment =
  "flex items-center justify-center gap-1.5 rounded-sm border border-transparent text-sm text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none"
const segmentOn = "border-border-strong bg-popover font-medium text-foreground"

type Kind = "inline" | "shared" | "global"
const kinds = [
  { value: "inline", label: "Inline", icon: PencilLineIcon },
  { value: "shared", label: "Shared", icon: ListTodoIcon },
  { value: "global", label: "Global", icon: GlobeIcon },
] as const

/** Inline instructions for this step only, a shared task from this project, or a global task from ~/.config/unelos. */
export function TaskSourceField({
  root,
  form,
  index,
  task,
}: {
  root: string
  form: WorkflowForm
  index: number
  task: StepTask
}) {
  const fetchShared = useCallback(() => listTasks(root), [root])
  const lists = {
    shared: useLazy(fetchShared),
    global: useLazy(listGlobalTasks),
  }
  const discard = useDiscardChanges()
  const [open, setOpen] = useState(false)
  const [picking, setPicking] = useState<"shared" | "global" | null>(null)
  const [error, setError] = useState<string>()
  const ref = task.source === "named" && task.id ? taskRef(task.id) : null
  const current: Kind = ref ? (ref.global ? "global" : "shared") : "inline"
  const active = picking ?? current
  const list = active === "inline" ? null : lists[active]
  // The picker trigger only shows the stored task while its kind is the active one.
  const chosen = ref && active === current ? ref.name : null

  const openPicker = (next: boolean) => {
    setOpen(next)
    if (next) list?.load()
    else setPicking(null)
  }
  const pick = async (kind: "shared" | "global", name: string) => {
    setOpen(false)
    setPicking(null)
    setError(undefined)
    if (
      current === "inline" &&
      (task.instructions.trim() || task.skills.length) &&
      !(await discard.ask(
        `Replace this step’s inline instructions, skills and outcomes with the ${kind} task?`
      ))
    )
      return
    try {
      const doc =
        kind === "global"
          ? await getGlobalTask(name)
          : await getTask(root, name)
      form.setFieldValue(`steps[${index}].task`, {
        source: "named",
        id: kind === "global" ? `global/${doc.id}` : doc.id,
        description: doc.description,
        instructions: doc.instructions,
        skills: doc.skills,
        outcomes: doc.outcomes,
      })
    } catch (e) {
      setError(`Couldn’t load ${kind} task ${name}: ${String(e)}`)
    }
  }
  const choose = (kind: Kind) => {
    if (kind !== "inline") {
      setPicking(kind)
      setOpen(true)
      lists[kind].load()
    } else if (ref) {
      // Fork: keep the task's content as editable inline content; routes already match its outcomes.
      setPicking(null)
      form.setFieldValue(`steps[${index}].task`, {
        ...task,
        source: "inline",
        id: null,
      })
    } else setPicking(null)
  }

  return (
    <Row
      label="Task"
      labelId="step-task-label"
      labelClassName="self-start flex h-8 items-center"
      errors={error ? [{ message: error }] : []}
      errorId="step-task-error"
    >
      <div className="flex min-w-0 flex-col gap-1.5">
        <div
          role="group"
          aria-labelledby="step-task-label"
          className="grid h-8 grid-cols-3 gap-0.5 rounded-md bg-muted p-0.5"
        >
          {kinds.map(({ value, label, icon: Icon }) => {
            const on = active === value
            return (
              <button
                key={value}
                type="button"
                aria-pressed={on}
                onClick={() => choose(value)}
                className={cn(segment, on && segmentOn)}
              >
                <Icon
                  aria-hidden="true"
                  className={cn("size-3.5", on && "text-primary")}
                />
                {label}
              </button>
            )
          })}
        </div>
        {active !== "inline" && list && (
          <div className="flex min-w-0 items-center gap-1">
            <Popover open={open} onOpenChange={openPicker}>
              <PopoverTrigger
                aria-label={active === "global" ? "Global task" : "Shared task"}
                render={
                  <Button
                    variant="outline"
                    className="h-auto min-w-0 flex-1 justify-start py-1.5 font-normal"
                  />
                }
              >
                {active === "global" ? (
                  <GlobeIcon
                    aria-hidden="true"
                    className="text-muted-foreground"
                  />
                ) : (
                  <ListTodoIcon
                    aria-hidden="true"
                    className="text-muted-foreground"
                  />
                )}
                {chosen ? (
                  <span className="flex min-w-0 flex-1 flex-col text-left">
                    <span className="truncate">{humanize(chosen)}</span>
                    <span className="truncate font-mono text-xs text-muted-foreground">
                      {active === "global" ? "Global task" : `tasks/${chosen}`}
                    </span>
                  </span>
                ) : (
                  <span className="flex-1 text-left text-muted-foreground">
                    Select {active} task
                  </span>
                )}
                <ChevronsUpDownIcon
                  aria-hidden="true"
                  className="text-muted-foreground"
                />
              </PopoverTrigger>
              <PopoverContent
                align="start"
                className="w-(--anchor-width) min-w-64 p-0"
              >
                <Command>
                  <CommandInput placeholder={`Search ${active} tasks…`} />
                  <CommandList>
                    {list.data ? (
                      <>
                        <CommandEmpty>
                          {list.data.length
                            ? "No tasks match."
                            : active === "global"
                              ? "No global tasks on this machine."
                              : "No shared tasks in this project."}
                        </CommandEmpty>
                        <CommandGroup>
                          {list.data.map((t) => (
                            <CommandItem
                              key={t.name}
                              value={t.name}
                              disabled={!!t.problem}
                              data-checked={t.name === chosen}
                              onSelect={() => void pick(active, t.name)}
                            >
                              <span className="flex min-w-0 flex-1 flex-col">
                                <span className="truncate">
                                  {humanize(t.name)}
                                </span>
                                <span className="truncate text-xs text-muted-foreground">
                                  {t.problem ?? t.description ?? t.name}
                                </span>
                              </span>
                            </CommandItem>
                          ))}
                        </CommandGroup>
                      </>
                    ) : (
                      <PickerStatus
                        what={`${active} tasks`}
                        error={list.error}
                        onRetry={list.load}
                      />
                    )}
                  </CommandList>
                </Command>
              </PopoverContent>
            </Popover>
            {chosen && (
              <Link
                to={active === "global" ? "/global-tasks" : "/tasks"}
                search={{ task: chosen }}
                aria-label={`Open task ${chosen} in ${active === "global" ? "Global tasks" : "Tasks"}`}
                title={
                  active === "global" ? "Open in Global tasks" : "Open in Tasks"
                }
                className={buttonVariants({
                  variant: "ghost",
                  size: "icon-sm",
                })}
              >
                <ExternalLinkIcon aria-hidden="true" />
              </Link>
            )}
          </div>
        )}
      </div>
      {discard.dialog}
    </Row>
  )
}
