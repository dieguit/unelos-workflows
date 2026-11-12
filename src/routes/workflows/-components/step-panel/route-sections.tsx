import { ArrowRightIcon, ChevronsUpDownIcon } from "lucide-react"
import { cn } from "cn"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import type { WorkflowStep } from "@/lib/projects"
import type { WorkflowForm } from "../../-hooks/use-workflow-form"
import { StepPicker } from "../step-picker"
import { ErrorText, InlineAdd, RemoveButton, rowFrame, Section, shown } from "@/components/step-panel/panel-ui"
import { reassignRoute } from "@/lib/step-edit"

type Props = { form: WorkflowForm; index: number; showErrors: boolean }

/** Inline task outcomes live in the step's routes while editing; Save derives the task definition from them. */
export function OutcomesSection({ form, index, showErrors }: Props) {
  return (
    <form.Field name={`steps[${index}].routes`} mode="array">
      {(field) => {
        const routes = field.state.value
        const declared = routes.filter((r) => r.outcome !== null).length
        return (
          <Section label="Outcomes">
            {routes.map((route, j) =>
              route.outcome === null ? null : (
                <form.Field key={j} name={`steps[${index}].routes[${j}].outcome`}>
                  {(outcome) => {
                    const errors = shown(outcome.state.meta, showErrors)
                    const errorId = `step-outcome-${j}-error`
                    return (
                      <div className="flex flex-col gap-1.5">
                        <div className="flex items-center gap-1">
                          <Input
                            aria-label={`Outcome ${j + 1}`}
                            autoFocus={outcome.state.value === "" && !outcome.state.meta.isTouched}
                            placeholder="outcome_name"
                            spellCheck={false}
                            autoComplete="off"
                            value={outcome.state.value ?? ""}
                            aria-invalid={errors.length > 0}
                            aria-describedby={errors.length ? errorId : undefined}
                            className="h-8 font-mono text-xs"
                            onBlur={outcome.handleBlur}
                            onChange={(e) => outcome.handleChange(e.target.value)}
                          />
                          <RemoveButton
                            label={`Remove outcome ${route.outcome || j + 1}`}
                            onClick={() =>
                              declared === 1
                                ? field.handleChange([{ outcome: null, target: "" }])
                                : field.removeValue(j)
                            }
                          />
                        </div>
                        <ErrorText id={errorId} errors={errors} />
                      </div>
                    )
                  }}
                </form.Field>
              )
            )}
            <InlineAdd
              label="Add outcome"
              onClick={() =>
                declared === 0
                  ? field.handleChange([{ outcome: "", target: "" }])
                  : field.pushValue({ outcome: "", target: "" })
              }
            />
          </Section>
        )
      }}
    </form.Field>
  )
}

/** Read-only outcome list for steps that use a named task. */
export function NamedOutcomes({ outcomes, source }: { outcomes: string[]; source: string | null }) {
  return (
    <Section label="Outcomes" source={source}>
      {outcomes.length ? (
        outcomes.map((o) => (
          <div key={o} className={cn(rowFrame, "border-transparent bg-muted font-mono text-xs text-foreground-dim")}>
            {o}
          </div>
        ))
      ) : (
        <p className="text-sm text-muted-foreground">None declared</p>
      )}
    </Section>
  )
}

/** Outcome → destination rows, or a single Next step when nothing is declared. Outcome names are not edited here. */
export function RoutesSection({ form, index, showErrors, step, steps }: Props & { step: WorkflowStep; steps: WorkflowStep[] }) {
  const fixed = step.kind === "human-gate" || step.task?.source === "named"
  const expected: (string | null)[] | null = step.task?.source === "named" ? (step.task.outcomes.length ? step.task.outcomes : [null]) : null
  const missing = expected?.filter((outcome) => !step.routes.some((r) => r.outcome === outcome)) ?? []
  const mismatch = expected && (missing.length > 0 || step.routes.some((r) => !expected.includes(r.outcome)))
  if (mismatch) return (
    <Section label="Repair routes">
      <p className="text-xs text-muted-foreground">Routes no longer match this task. Reassign or remove outdated routes, and add missing routes.</p>
      {step.routes.map((route, j) => {
        const stale = !expected.includes(route.outcome)
        return <div key={j} className="flex flex-col gap-1.5">
          <div className="flex items-center gap-2">
            {stale ? <Popover>
              <PopoverTrigger aria-label={`Reassign ${route.outcome ?? "next step"}`} render={<Button variant="outline" className="h-8 w-39 min-w-0 justify-between font-mono text-xs font-normal" />}>
                <span className="truncate">{route.outcome ?? "Next step"} (outdated)</span>
                <ChevronsUpDownIcon aria-hidden="true" className="text-muted-foreground" />
              </PopoverTrigger>
              <PopoverContent align="start" className="w-(--anchor-width) min-w-39 p-1">
                {missing.length ? missing.map((outcome, i) => <Button key={i} variant="ghost" className="w-full justify-start font-mono text-xs font-normal" onClick={() => form.setFieldValue(`steps[${index}].routes`, reassignRoute(step.routes, j, outcome))}>{outcome ?? "Next step"}</Button>) : <p className="p-2 text-xs text-muted-foreground">No missing outcomes to assign.</p>}
              </PopoverContent>
            </Popover> : <span className={cn(rowFrame, "w-39 shrink-0 border-transparent bg-muted font-mono text-xs text-foreground-dim")}>{route.outcome ?? "Next step"}</span>}
            <ArrowRightIcon aria-hidden="true" className="size-3.5 shrink-0 text-muted-foreground" />
            <Destination form={form} index={index} route={j} showErrors={showErrors} steps={steps} step={step} label={`Destination for ${route.outcome ?? "next step"}`} />
            {stale && <RemoveButton label={`Remove outdated route ${route.outcome ?? "next step"}`} onClick={() => form.setFieldValue(`steps[${index}].routes`, step.routes.filter((_, i) => i !== j))} />}
          </div>
          <RouteError form={form} index={index} route={j} showErrors={showErrors} />
        </div>
      })}
      {missing.map((outcome, i) => <Button key={i} variant="ghost" size="sm" className="self-start" onClick={() => form.setFieldValue(`steps[${index}].routes`, [...step.routes, { outcome, target: "" }])}>Add route: {outcome ?? "Next step"}</Button>)}
    </Section>
  )
  const declared = step.routes.some((r) => r.outcome !== null)
  if (!declared)
    return (
      <Section label="Next step">
        {/* shortcut: a step with no routes at all shows no picker; add one here if such files appear. */}
        {step.routes.length > 0 && (
          <>
            <Destination form={form} index={index} route={0} showErrors={showErrors} steps={steps} step={step} label="Next step" />
            <RouteError form={form} index={index} route={0} showErrors={showErrors} />
          </>
        )}
      </Section>
    )
  return (
    <Section label="Routes">
      {step.routes.map((route, j) => (
        <div key={j} className="flex flex-col gap-1.5">
          <div className="flex items-center gap-2">
            <span
              className={cn(
                rowFrame,
                "w-39 shrink-0 font-mono text-xs",
                fixed && "border-transparent bg-muted text-foreground-dim",
                !route.outcome && "text-muted-foreground italic"
              )}
            >
              <span className="truncate">{route.outcome || "unnamed"}</span>
            </span>
            <ArrowRightIcon aria-hidden="true" className="size-3.5 shrink-0 text-muted-foreground" />
            <Destination
              form={form}
              index={index}
              route={j}
              showErrors={showErrors}
              steps={steps}
              step={step}
              label={`Destination for ${route.outcome || `outcome ${j + 1}`}`}
            />
          </div>
          <RouteError form={form} index={index} route={j} showErrors={showErrors} />
        </div>
      ))}
    </Section>
  )
}

type RouteProps = Props & { route: number }

function Destination({
  form,
  index,
  route,
  showErrors,
  steps,
  step,
  label,
}: RouteProps & { steps: WorkflowStep[]; step: WorkflowStep; label: string }) {
  const id = `step-route-${route}`
  return (
    <form.Field name={`steps[${index}].routes[${route}].target`}>
      {(field) => {
        const invalid = shown(field.state.meta, showErrors).length > 0
        return (
          <div role="group" aria-label={label} className="min-w-0 flex-1">
            <StepPicker
              id={id}
              variant="field"
              purpose="destination"
              exclude={step.id}
              steps={steps}
              value={field.state.value}
              onChange={field.handleChange}
              onBlur={field.handleBlur}
              invalid={invalid}
              describedBy={invalid ? `${id}-error` : undefined}
            />
          </div>
        )
      }}
    </form.Field>
  )
}

/** Shown under the whole row so long messages do not squeeze the picker. */
function RouteError({ form, index, route, showErrors }: RouteProps) {
  return (
    <form.Field name={`steps[${index}].routes[${route}].target`}>
      {(field) => <ErrorText id={`step-route-${route}-error`} errors={shown(field.state.meta, showErrors)} />}
    </form.Field>
  )
}
