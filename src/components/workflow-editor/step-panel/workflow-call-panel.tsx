import { useEffect, useState } from "react"
import { useStore } from "@tanstack/react-form"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { StepIconBox } from "@/components/flow/step-card"
import { PanelBody, PanelHeader, PanelTabs, RemoveStep } from "@/components/step-panel/panel-chrome"
import { InlineAdd, RemoveButton, Section } from "@/components/step-panel/panel-ui"
import { getWorkflow, listWorkflows, type StepCall, type WorkflowDocument, type WorkflowStep, type WorkflowSummary } from "@/lib/projects"
import { callableWorkflows, childContract, humanize, producers } from "@/lib/step-edit"
import type { WorkflowForm } from "../../-hooks/use-workflow-form"
import { NameField } from "./basic-fields"
import { RoutesSection } from "./route-sections"

export const withCall = (step: WorkflowStep, call: StepCall): WorkflowStep => ({
  ...step, call, inputs: Object.values(call.inputs), outputs: Object.values(call.outputs),
})

export function WorkflowCallPanel({
  root, current, form, index, showErrors, disabled, nameDraft, onNameDraft, onClose, onRemove,
}: {
  root: string
  current: string
  form: WorkflowForm
  index: number
  showErrors: boolean
  disabled: boolean
  nameDraft?: { text: string; error: string }
  onNameDraft: (id: string, draft?: { text: string; error: string }) => void
  onClose: () => void
  onRemove: () => void
}) {
  const draft = useStore(form.store, (s) => s.values)
  const step = draft.steps[index]
  const workflow = step.call?.workflow ?? ""
  const [summaries, setSummaries] = useState<WorkflowSummary[]>()
  const [child, setChild] = useState<WorkflowDocument>()
  const [error, setError] = useState<string>()
  const [attempt, setAttempt] = useState(0)
  useEffect(() => {
    let active = true
    listWorkflows(root).then(
      (items) => { if (active) setSummaries(items) },
      (err) => { if (active) setError(String(err)) },
    )
    return () => { active = false }
  }, [root, attempt])
  useEffect(() => {
    if (!workflow) return
    let active = true
    getWorkflow(root, workflow).then((doc) => {
      if (!active) return
      setChild(doc)
      const target = form.state.values.steps[index]
      if (!target.routes.length) {
        const { ends } = childContract(doc)
        form.setFieldValue(`steps[${index}].routes`, ends.map((end) => ({
          outcome: ends.length === 1 ? null : end, target: "",
        })))
      }
    }, (err) => { if (active) setError(String(err)) })
    return () => { active = false }
  }, [root, workflow, form, index, attempt])
  const contract = child?.id === workflow ? childContract(child) : undefined
  const call = step.call ?? { workflow, inputs: {}, outputs: {} }
  const update = (next: StepCall) => form.setFieldValue(`steps[${index}]`, withCall(form.state.values.steps[index], next))
  const options = [...new Set([...draft.inputs, ...producers(draft.steps, index).map((p) => p.artifact)])]
  const available = callableWorkflows(current, summaries ?? [])
  return <>
    <PanelHeader icon={<StepIconBox kind="workflow" />} title={humanize(step.id)} subtitle={`Workflow · id ${step.id}`} onClose={onClose} />
    <PanelTabs />
    <PanelBody disabled={disabled} footer={<RemoveStep step={step} disabled={disabled} onRemove={onRemove} />}>
      <NameField form={form} index={index} showErrors={showErrors} step={step} nameDraft={nameDraft} onNameDraft={onNameDraft} />
      <Section label="Workflow">
        <select aria-label="Called workflow" className="h-8 w-full rounded-md border bg-background px-2 text-sm" value={workflow} disabled={!summaries} onChange={(e) => {
          setError(undefined)
          update({ workflow: e.target.value, inputs: {}, outputs: {} })
          form.setFieldValue(`steps[${index}].routes`, [])
        }}>
          <option value="">Select workflow</option>
          {workflow && !available.includes(workflow) && <option value={workflow} disabled>{workflow} (unavailable)</option>}
          {available.map((name) => <option key={name} value={name}>{name}</option>)}
        </select>
        {!summaries && !error && <p role="status">Loading workflows…</p>}
      </Section>
      {error && <div role="alert"><p className="text-sm text-destructive">{error}</p><Button variant="outline" size="sm" onClick={() => { setError(undefined); setAttempt((n) => n + 1) }}>Retry</Button></div>}
      {workflow && !contract && !error && <p role="status">Loading child contract…</p>}
      {contract && <>
        <Section label="Inputs">
          {contract.inputs.map((input) => {
            const mapped = Object.hasOwn(call.inputs, input) ? call.inputs[input] : ""
            return (
              <label key={input} className="flex flex-col gap-1 text-xs">
                <span className="font-mono">{input}</span>
                <select aria-label={`Parent artifact for ${input}`} required className="h-8 rounded-md border bg-background px-2" value={mapped} onChange={(e) => update({ ...call, inputs: { ...call.inputs, [input]: e.target.value } })}>
                  <option value="">Select parent artifact</option>
                  {mapped && !options.includes(mapped) && <option value={mapped}>{mapped} (unavailable)</option>}
                  {options.map((name) => <option key={name} value={name}>{name}</option>)}
                </select>
              </label>
            )
          })}
        </Section>
        <Section label="Outputs">
          {Object.entries(call.outputs).map(([artifact, exposed]) => <div key={artifact} className="flex items-center gap-1">
            <select aria-label={`Child artifact ${artifact}`} className="h-8 min-w-0 flex-1 rounded-md border bg-background text-xs" value={artifact} onChange={(e) => update({ ...call, outputs: Object.fromEntries(Object.entries(call.outputs).map(([key, value]) => [key === artifact ? e.target.value : key, value])) })}>
              {!contract.artifacts.includes(artifact) && <option value={artifact}>{artifact} (unavailable)</option>}
              {contract.artifacts.filter((name) => name === artifact || !Object.hasOwn(call.outputs, name)).map((name) => <option key={name} value={name}>{name}</option>)}
            </select>
            <Input aria-label={`Exposed name for ${artifact}`} className="min-w-0 flex-1 font-mono text-xs" value={exposed} onChange={(e) => update({ ...call, outputs: { ...call.outputs, [artifact]: e.target.value } })} />
            <RemoveButton label={`Remove output ${artifact}`} onClick={() => update({ ...call, outputs: Object.fromEntries(Object.entries(call.outputs).filter(([key]) => key !== artifact)) })} />
          </div>)}
          {contract.artifacts.some((name) => !Object.hasOwn(call.outputs, name)) && <InlineAdd label="Expose output" onClick={() => {
            const artifact = contract.artifacts.find((name) => !Object.hasOwn(call.outputs, name))!
            update({ ...call, outputs: { ...call.outputs, [artifact]: artifact } })
          }} />}
        </Section>
      </>}
      <RoutesSection form={form} index={index} showErrors={showErrors} step={step} steps={draft.steps} />
    </PanelBody>
  </>
}
