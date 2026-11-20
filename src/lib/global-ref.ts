import {
  listAgents,
  listGlobalAgents,
  listGlobalWorkflows,
  listWorkflows,
  type AgentSummary,
  type WorkflowSummary,
} from "@/lib/projects"

const PREFIX = "global/"

export const isGlobalRef = (ref: string) => ref.startsWith(PREFIX)
export const stripGlobal = (ref: string) => (isGlobalRef(ref) ? ref.slice(PREFIX.length) : ref)
export const globalRef = (id: string) => PREFIX + stripGlobal(id)

/** Project + global agents (global ones labelled `global/<id>`), or only global ones when `root` is null. */
export async function listAgentChoices(root: string | null): Promise<AgentSummary[]> {
  const [project, global] = await Promise.all([root === null ? [] : listAgents(root), listGlobalAgents()])
  return [...project, ...global.map((a) => ({ ...a, name: globalRef(a.name) }))]
}

/** Global workflows as `global/<id>`, including the references they call. */
export async function listGlobalWorkflowChoices(): Promise<WorkflowSummary[]> {
  return (await listGlobalWorkflows()).map((w) => ({
    ...w,
    global: true,
    name: globalRef(w.name),
    calls: w.calls.map((c) => (c.includes("/") ? c : globalRef(c))),
  }))
}

/** Workflows a step can call: project ones plus global ones, or only global ones when `root` is null. */
export const listWorkflowChoices = (root: string | null) =>
  root === null ? listGlobalWorkflowChoices() : listWorkflows(root)
