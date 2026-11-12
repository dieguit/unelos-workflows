import { mockIPC } from "@tauri-apps/api/mocks"
import type { Harness } from "./harnesses"
import type { ModelMappings } from "./model-mappings"
import type { McpDocument, TaskDocument, AgentDocument, AgentSummary, ProjectState, SkillDocument, SkillSummary, WorkflowDocument, WorkflowDraft, WorkflowStep, WorkflowIssue, WorkflowSummary } from "./projects"

const home = "/home/dieguit"
const day = 86_400_000
const mode = new URLSearchParams(location.search).get("mock")
const pick = new URLSearchParams(location.search).get("pick")

const state: ProjectState = {
  home,
  recents:
    mode === "none"
      ? []
      : [
          { path: `${home}/projects/unelos/unelos-app`, name: "unelos-app", openedAt: Date.now(), available: mode !== "unavailable" },
          { path: `${home}/work/api-gateway`, name: "api-gateway", openedAt: Date.now() - day, available: true },
          { path: `${home}/projects/docs-site`, name: "docs-site", openedAt: Date.now() - 3 * day, available: true },
        ],
}

const workflows: WorkflowSummary[] =
  mode === "empty"
    ? []
    : [
        { name: "Implement feature", file: "implement-feature.yaml", description: "Plan, approve, implement and review a feature with a correction loop.", steps: 5, problem: null },
        { name: "Investigate bug", file: "investigate-bug.yaml", description: "Reproduce the issue, locate the root cause and propose a fix with evidence.", steps: 4, problem: null },
        { name: "Review change", file: "review-change.yaml", description: "Independent review of a diff with a findings report and human sign-off.", steps: 3, problem: "nodes.review.agent: Agent reference missing" },
        { name: "Refactor module", file: "refactor-module.yaml", description: "Map affected code, refactor in parallel slices and verify behavior is unchanged.", steps: 6, problem: null },
      ]

const agents: AgentSummary[] = [
  { name: "broken", model: null, skills: [], mcps: [], problem: "model: expected a string" },
  { name: "engineer", model: "smart", skills: ["rust", "typescript"], mcps: ["github"], problem: null },
  { name: "reviewer", model: null, skills: [], mcps: [], problem: null },
]

// Mock fixtures only: production validation remains in Rust.
const workflowSteps: Record<string, Pick<WorkflowStep, "id" | "label" | "kind">[]> = {
  "implement-feature": [
    { id: "plan", label: "Plan", kind: "task" },
    { id: "approve-plan", label: "Approve plan", kind: "human-gate" },
    { id: "implement", label: "Implement", kind: "parallel" },
    { id: "review", label: "Review", kind: "task" },
    { id: "done", label: "Done", kind: "end" },
  ],
  "investigate-bug": [
    { id: "reproduce", label: "Reproduce", kind: "task" },
    { id: "investigate", label: "Investigate", kind: "task" },
    { id: "approve", label: "Approve", kind: "human-gate" },
    { id: "done", label: "Done", kind: "end" },
  ],
  "review-change": [
    { id: "review", label: "Review", kind: "task" },
    { id: "sign-off", label: "Sign off", kind: "human-gate" },
    { id: "done", label: "Done", kind: "end" },
  ],
  "refactor-module": [
    { id: "map", label: "Map", kind: "task" },
    { id: "refactor", label: "Refactor", kind: "parallel" },
    { id: "verify", label: "Verify", kind: "task" },
    { id: "review", label: "Review", kind: "task" },
    { id: "approve", label: "Approve", kind: "human-gate" },
    { id: "done", label: "Done", kind: "end" },
  ],
}
const projectSettings = new Map<string, boolean | null>()
const projectWorkflows = new Map<string, { summary: WorkflowSummary; document: WorkflowDocument }[]>()
function workflowsFor(root: string) {
  if (!state.recents.some((r) => r.path === root && r.available)) throw "Project is not approved"
  if (mode === "error") throw "missing file /x/.unelos/project.yaml"
  if (!projectWorkflows.has(root)) {
    const prefix = mode === "switch" ? `${root.split("/").pop()}-` : ""
    projectWorkflows.set(root, workflows.map((summary) => {
      const original = summary.file.replace(/\.yaml$/, "")
      const id = prefix + original
       const steps: WorkflowStep[] = (workflowSteps[original] ?? []).map((step, index, all) => ({
         ...step, originalId: step.id, task: step.kind === "task" ? { source: "inline", id: null, description: null, instructions: `Perform ${step.id}`, skills: [], outcomes: [] } : null, label: step.id, title: step.kind === "task" ? step.id : null,
         agent: step.kind === "task" ? "engineer" : null, execution: step.kind === "task" ? "main" : null,
         routes: all[index + 1] ? [{ outcome: null, target: all[index + 1].id }] : [], workers: [], inputs: [], outputs: [],
       }))
       if (original === "implement-feature") {
         Object.assign(steps[0], { title: "write-plan", agent: "planner" })
         Object.assign(steps[1], { inputs: ["plan.md"], routes: [{ outcome: "approved", target: "implement" }, { outcome: "rejected", target: "plan" }] })
          steps[2].workers = [
            { task: { source: "inline", id: null, description: "Implement API", instructions: "Implement and test the API described in plan.md.", skills: ["rust"], outcomes: [] }, agent: "engineer" },
            { task: { source: "inline", id: null, description: "Implement UI", instructions: "Implement the UI from plan.md and verify the user flow.", skills: ["code-review"], outcomes: [] }, agent: "engineer" },
          ]
          Object.assign(steps[2], { inputs: ["plan.md"], outputs: ["implementation-report.md"] })
         Object.assign(steps[3], { title: "review-change", agent: "reviewer", execution: "subagent", routes: [{ outcome: "approved", target: "done" }, { outcome: "changes_requested", target: "implement" }] })
       }
        if (original === "refactor-module") steps[1].workers = [
          { task: { source: "inline", id: null, description: "Refactor module", instructions: "Refactor the module while preserving its public API.", skills: ["rust"], outcomes: [] }, agent: "engineer" },
          { task: { source: "named", id: "review-change", description: "Review the refactor", instructions: "Review the refactor for regressions.", skills: ["code-review"], outcomes: [] }, agent: "reviewer" },
        ]
        for (const step of steps) if (step.task) step.task.outcomes = step.routes.flatMap((route) => route.outcome ? [route.outcome] : [])
       return {
        summary: { ...summary, name: mode === "switch" ? `${root.split("/").pop()}: ${summary.name}` : summary.name, file: `${id}.yaml` },
        document: { id, name: id, description: summary.description, start: steps[0]?.id ?? "", steps, issues: summary.problem ? [{ message: summary.problem, step: "review" }] : steps.length ? [] : [{ message: "Mock fixture has no step definitions", step: null }] },
      }
    }))
  }
  return projectWorkflows.get(root)!
}
function workflowFor(root: string, id: string) {
  const workflow = workflowsFor(root).find((w) => w.document.id === id)
  if (!workflow) throw `Workflow ${id} could not be loaded`
  return workflow
}
function workflowIssues(root: string, id: string, settings: WorkflowDraft): WorkflowIssue[] {
  const { document } = workflowFor(root, id)
  if (settings.name !== id) {
    if (!/^[A-Za-z0-9_-]{1,128}$/.test(settings.name)) return [{ message: "Name must contain 1–128 ASCII letters, digits, underscores or hyphens", step: null }]
    if (workflowsFor(root).some((w) => w.document.id === settings.name)) return [{ message: `Workflow ${settings.name} already exists`, step: null }]
  }
  if (!settings.steps.some((step) => step.id === settings.start)) return [{ message: `start: unknown target ${JSON.stringify(settings.start)}`, step: null }]
   const missing = settings.steps.filter((step) => step.kind === "task" && !step.agent?.trim())
   return [...document.issues.filter((issue) => issue.message !== "Pick an agent"), ...missing.map((step) => ({ message: "Pick an agent", step: step.id }))]
}

const agentDocuments: Record<string, AgentDocument> = Object.fromEntries(
  agents.filter((a) => !a.problem).map((a) => [a.name, {
    id: a.name, instructions: a.name === "reviewer" ? "Inspect requirements and actual changes independently." : "Implement the requested changes.",
    skills: [...a.skills], mcps: [...a.mcps], tools: a.name === "engineer" ? ["read-file", "run-command"] : [], model: a.model,
  }])
)

const skills: SkillSummary[] = [
  { name: "broken", description: null, problem: "frontmatter: missing closing ---" },
  { name: "code-review", description: "Review code for correctness and missing tests.", problem: null },
  { name: "planning", description: "Write a concrete implementation plan from the request.", problem: null },
  { name: "rust", description: null, problem: null },
]

const globalMcps: McpDocument[] = []

const mcps: McpDocument[] = [
  { id: "github", type: "remote", url: "https://api.githubcopilot.com/mcp/", headers: { Authorization: "Bearer {env:GITHUB_TOKEN}" } },
  { id: "filesystem", type: "local", command: ["npx", "-y", "@modelcontextprotocol/server-filesystem", "."], environment: {} },
]

const bodies: Record<string, string> = {
  "code-review": "# Code review\n\nInspect the actual changes, not a summary of them.\n\n## Correctness\n- Read the diff against the plan before judging it.\n- Check edge cases, error handling and regressions.\n\n## Tests\n- Confirm changed behavior is covered by tests.\n- Flag missing or weak tests with file and line.\n\nReport findings, then select an outcome.\n",
  planning: "# Planning\n\nWrite a concrete, ordered plan before changing code.\n",
  rust: "# Rust\n\nFollow existing Rust conventions in this project.\n",
}

const globalTasks: Record<string, TaskDocument> = {}
const projectTasks = new Map<string, Record<string, TaskDocument>>()

const projectAuthoring = new Map<string, { agents: AgentSummary[]; agentDocuments: Record<string, AgentDocument>; skills: SkillSummary[]; bodies: Record<string, string> }>()
function authoringFor(root: string) {
  if (!projectAuthoring.has(root)) projectAuthoring.set(root, structuredClone({ agents, agentDocuments, skills, bodies }))
  return projectAuthoring.get(root)!
}

const harness = (id: string, name: string, version: string | null = null): Harness => ({
  id, name, installable: id === "opencode", detected: version !== null, installed: false, updateAvailable: false,
  version, integrationVersion: null, binary: version ? `${home}/.local/bin/${id}` : null, configDir: version ? `${home}/.config/${id}` : null,
})
const harnesses: Harness[] = [
  { ...harness("opencode", "OpenCode", "1.0.4"), installed: true, updateAvailable: mode === "update", integrationVersion: "0.0.1" },
  harness("claude", "Claude Code", "2.3.0"),
  harness("codex", "Codex", "0.48.1"),
  harness("pi", "Pi", "0.9.3"),
  ...[["copilot", "Copilot"], ["cursor", "Cursor"], ["droid", "Droid"], ["kimi", "Kimi"], ["qwen", "Qwen Code"], ["gemini", "Gemini CLI"], ["amp", "Amp"], ["cline", "Cline"]].map(([id, name]) => harness(id, name)),
]

const modelMappings: ModelMappings = {
  smart: { opencode: "anthropic/claude-sonnet-4-5" },
  balanced: {},
  fast: { opencode: "openai/gpt-5-mini" },
}
const harnessModels = {
  anthropic: ["claude-opus-4-1", "claude-opus-4", "claude-sonnet-4-5", "claude-sonnet-4", "claude-3-7-sonnet", "claude-3-5-haiku"],
  openai: ["gpt-5", "gpt-5-mini", "gpt-5-nano", "gpt-4.1", "gpt-4.1-mini", "gpt-4.1-nano", "o3", "o3-mini", "o4-mini"],
  google: ["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.5-flash-lite", "gemini-2.0-flash"],
  deepseek: ["deepseek-chat", "deepseek-reasoner"],
  "amazon-bedrock": ["anthropic.claude-sonnet-4", "anthropic.claude-opus-4", "amazon.nova-pro-v1"],
  mistral: ["mistral-large-latest", "codestral-latest", "devstral-small-latest"],
  groq: ["llama-3.3-70b-versatile", "qwen/qwen3-32b", "openai/gpt-oss-120b"],
}

export function installDevIpcMock() {
  mockIPC(async (cmd, args) => {
    const path = (args as { path?: string })?.path
    const root = (args as { root?: string })?.root ?? state.recents[0]?.path ?? ""
    const { agents, agentDocuments, skills, bodies } = authoringFor(root)
    const skillDoc = (id: string): SkillDocument => {
      const skill = skills.find((s) => s.name === id)
      if (!skill) throw `Skill ${id} not found`
      if (skill.problem) throw `${id}/SKILL.md: ${skill.problem}`
      return { name: id, description: skill.description, body: bodies[id] ?? "" }
    }
    const tasks = cmd.includes("global_task") ? globalTasks : (projectTasks.get(root) ?? (() => {
      const documents: Record<string, TaskDocument> = {}
      projectTasks.set(root, documents)
      return documents
    })())
    switch (cmd) {
      case "list_tasks":
      case "list_global_tasks":
        return Object.values(tasks).sort((a, b) => a.id.localeCompare(b.id)).map((task) => ({ name: task.id, description: task.description, skills: [...task.skills], outcomes: [...task.outcomes], problem: null }))
      case "get_task":
      case "get_global_task": {
        const { id } = args as { id: string }
        if (!Object.hasOwn(tasks, id)) throw `Task ${id} not found`
        return structuredClone(tasks[id])
      }
      case "save_task":
      case "save_global_task": {
        const { originalId, task, outcomeRenames = {} } = args as { originalId: string | null; task: TaskDocument; outcomeRenames?: Record<string, string> }
        if (!/^[A-Za-z0-9_-]{1,128}$/.test(task.id)) throw "Invalid Task ID"
        if (originalId !== null && originalId !== task.id) throw "Task IDs are read-only"
        if (originalId === null && Object.hasOwn(tasks, task.id)) throw "Already exists"
        if (originalId !== null && !Object.hasOwn(tasks, originalId)) throw "Task not found"
        if (cmd === "save_task") {
          const old = originalId === null ? [] : tasks[originalId].outcomes
          const oldSet = new Set(old), newSet = new Set(task.outcomes)
          const renamed = (outcome: string) => Object.hasOwn(outcomeRenames, outcome) ? outcomeRenames[outcome] : outcome
          if (oldSet.size !== old.length || newSet.size !== task.outcomes.length) throw "Duplicate task outcomes"
          if (Object.entries(outcomeRenames).some(([from, to]) => !oldSet.has(from) || !newSet.has(to))) throw "Invalid outcome renames"
          if (new Set(old.map(renamed)).size !== old.length) throw "Outcome renames collide"
          const changed = oldSet.size !== newSet.size || [...oldSet].some((outcome) => !newSet.has(outcome) || renamed(outcome) !== outcome)
          const steps = workflowsFor(root).flatMap((workflow) => workflow.document.steps).filter((step) => step.kind === "task" && step.task?.source === "named" && step.task.id === originalId)
          if (changed) for (const step of steps) {
            const routes = step.routes.flatMap((route) => route.outcome === null ? [] : [route.outcome])
            const mapped = routes.map(renamed)
            if (routes.length !== oldSet.size || routes.some((outcome) => !oldSet.has(outcome)) || new Set(mapped).size !== newSet.size || mapped.some((outcome) => !newSet.has(outcome)) || (task.outcomes.length === 0 && (step.routes.length !== 1 || step.routes[0].outcome !== null))) throw "Outcome changes require manual workflow route repair"
          }
          for (const step of steps) {
            for (const route of step.routes) if (route.outcome !== null) route.outcome = renamed(route.outcome)
            if (step.task) step.task.outcomes = [...task.outcomes]
          }
        }
        tasks[task.id] = structuredClone(task)
        return structuredClone(task)
      }
      case "duplicate_task":
      case "duplicate_global_task": {
        const { id, newId } = args as { id: string; newId: string }
        if (!Object.hasOwn(tasks, id)) throw "Task not found"
        if (!/^[A-Za-z0-9_-]{1,128}$/.test(newId)) throw "Invalid Task ID"
        if (Object.hasOwn(tasks, newId)) throw "Already exists"
        tasks[newId] = { ...structuredClone(tasks[id]), id: newId }
        return
      }
      case "remove_task":
      case "remove_global_task": {
        const { id } = args as { id: string }
        if (!Object.hasOwn(tasks, id)) throw "Task not found"
        delete tasks[id]
        return
      }
      case "duplicate_agent":
      case "duplicate_skill":
      case "duplicate_workflow": {
        workflowsFor(root) // Same approved-root guard as the real commands.
        const { id, newId } = args as { id: string; newId: string }
        if (cmd === "duplicate_skill") {
          if (!newId || newId.trim() !== newId || newId === "." || newId === ".." || /[/\\:\0]/.test(newId)) throw "Invalid Skill ID"
          const skill = skills.find((s) => s.name === id)
          if (!skill) throw `Skill ${id} not found`
          if (skills.some((s) => s.name === newId)) throw "Already exists"
          skills.push({ ...structuredClone(skill), name: newId })
          bodies[newId] = bodies[id] ?? ""
        } else {
          if (!/^[A-Za-z0-9_-]{1,128}$/.test(newId)) throw "Name must contain 1–128 ASCII letters, digits, underscores or hyphens"
          if (cmd === "duplicate_agent") {
            const agent = agents.find((a) => a.name === id)
            if (!agent) throw `Agent ${id} not found`
            if (agents.some((a) => a.name === newId)) throw "Already exists"
            agents.push({ ...structuredClone(agent), name: newId })
            if (Object.hasOwn(agentDocuments, id)) agentDocuments[newId] = { ...structuredClone(agentDocuments[id]), id: newId }
          } else {
            const workflow = workflowFor(root, id)
            if (workflowsFor(root).some((w) => w.document.id === newId)) throw "Already exists"
            workflowsFor(root).push({ summary: { ...structuredClone(workflow.summary), name: newId, file: `${newId}.yaml` }, document: { ...structuredClone(workflow.document), name: newId, id: newId } })
          }
        }
        return null
      }
      case "remove_agent":
      case "remove_skill":
      case "remove_workflow": {
        const workflows = workflowsFor(root)
        const { id } = args as { id: string }
        if (cmd === "remove_workflow") {
          const index = workflows.findIndex((w) => w.document.id === id)
          if (index === -1) throw `Workflow ${id} not found`
          if (id.endsWith("implement-feature")) throw "Cannot remove workflow: finish or remove its in-progress runs first (mock fixture)"
          workflows.splice(index, 1)
        } else if (cmd === "remove_agent") {
          const index = agents.findIndex((a) => a.name === id)
          if (index === -1) throw `Agent ${id} not found`
          if (["engineer", "reviewer"].includes(id) && workflows.length) throw "Cannot remove Agent: referenced by workflows (mock fixture)"
          agents.splice(index, 1)
          delete agentDocuments[id]
        } else {
          const index = skills.findIndex((s) => s.name === id)
          if (index === -1) throw `Skill ${id} not found`
          if (agents.some((a) => a.skills.includes(id)) || (id === "planning" && workflows.length)) throw "Cannot remove Skill: referenced by agents or workflow tasks (mock fixture)"
          skills.splice(index, 1)
          delete bodies[id]
        }
        return null
      }
      case "list_model_mappings":
        return structuredClone(modelMappings)
      case "save_model_mapping": {
        const { original, alias, models } = args as { original: string | null; alias: string; models: Record<string, string> }
        if (!/^[a-z0-9]+([-_.][a-z0-9]+)*$/.test(alias)) throw "Alias must be lowercase letters and digits separated by -, _ or ."
        const filtered = Object.fromEntries(Object.entries(models).filter(([, model]) => model !== ""))
        if (Object.values(filtered).some((model) => /\p{White_Space}/u.test(model))) throw "Model must not contain whitespace"
        if (original !== null && !Object.hasOwn(modelMappings, original)) throw "Original alias not found"
        if (original !== alias && Object.hasOwn(modelMappings, alias)) throw "Alias already exists"
        if (original !== null) delete modelMappings[original]
        Object.defineProperty(modelMappings, alias, { value: filtered, enumerable: true, configurable: true, writable: true })
        return structuredClone(modelMappings)
      }
      case "delete_model_mapping": {
        const { alias } = args as { alias: string }
        if (!Object.hasOwn(modelMappings, alias)) throw "Alias not found"
        delete modelMappings[alias]
        return structuredClone(modelMappings)
      }
      case "harness_models":
        await new Promise((resolve) => setTimeout(resolve, 350))
        if ((args as { id: string }).id !== "opencode") throw "Harness models unavailable"
        return Object.entries(harnessModels).flatMap(([provider, models]) => models.map((model) => `${provider}/${model}`)).sort()
      case "project_state":
        return structuredClone(state)
      case "pick_project":
      case "open_project": {
        if (cmd === "pick_project" && pick === "cancel") return null
        if (cmd === "pick_project" && pick === "plain") throw "plain is not a Unelos project (missing .unelos/project.yaml)"
        const selected = cmd === "pick_project" ? `${home}/work/api-gateway` : path
        const entry = state.recents.find((r) => r.path === selected)
        if (cmd === "open_project" && !entry) throw "Project is not approved; select it using Open folder…"
        if (entry && !entry.available) throw `${entry.name} couldn’t be opened: the folder no longer exists`
        state.recents = [
          { path: selected!, name: selected!.split("/").pop()!, openedAt: Date.now(), available: true },
          ...state.recents.filter((r) => r.path !== selected),
        ]
        return structuredClone(state)
      }
      case "remove_recent":
        state.recents = state.recents.filter((r) => r.path !== path)
        return structuredClone(state)
      case "list_workflows": {
        const root = (args as { root: string }).root
        if (mode === "switch") await new Promise((resolve) => setTimeout(resolve, 900))
        return structuredClone(workflowsFor(root).map((w) => w.summary))
      }
      case "get_workflow": {
        const { root, id } = args as { root: string; id: string }
        return structuredClone(workflowFor(root, id).document)
      }
      case "create_workflow": {
        const { root } = args as { root: string }
        const workflows = workflowsFor(root)
        let id = "new-workflow"
        for (let n = 2; workflows.some((w) => w.document.id === id); n++) id = `new-workflow-${n}`
        const document: WorkflowDocument = {
          id, name: id, description: null, start: "new-task",
          steps: [
            { id: "new-task", originalId: "new-task", label: "new-task", kind: "task", title: "new-task", agent: "", execution: "main", task: { source: "inline", id: null, description: null, instructions: "Change me!", skills: [], outcomes: [] }, routes: [{ outcome: null, target: "end" }], workers: [], inputs: [], outputs: [] },
            { id: "end", originalId: "end", label: "end", kind: "end", title: null, agent: null, execution: null, task: null, routes: [], workers: [], inputs: [], outputs: [] },
          ],
          issues: [{ message: "Pick an agent", step: "new-task" }],
        }
        workflows.push({ summary: { name: id, file: `${id}.yaml`, description: null, steps: 2, problem: null }, document })
        return id
      }
      case "validate_workflow": {
        const { root, id, draft } = args as { root: string; id: string; draft: WorkflowDraft }
        return workflowIssues(root, id, draft)
      }
      case "save_workflow": {
        const { root, id, draft } = args as { root: string; id: string; draft: WorkflowDraft }
        await new Promise((resolve) => setTimeout(resolve, 900))
        if (mode === "save-error") throw "Project is read-only (mock save failure)"
        const issues = workflowIssues(root, id, draft)
        if (issues.length) throw issues.map((issue) => issue.message).join("\n")
        const workflow = workflowFor(root, id)
        Object.assign(workflow.document, structuredClone(draft), { id: draft.name, steps: structuredClone(draft.steps).map((step) => ({ ...step, originalId: step.id })), issues: [] })
        Object.assign(workflow.summary, { name: draft.name, file: `${draft.name}.yaml`, description: draft.description, problem: null })
        return structuredClone(workflow.document)
      }
      case "list_agents": {
        const root = (args as { root: string }).root
        if (!state.recents.some((r) => r.path === root && r.available)) throw "Project is not approved"
        if (mode === "error") throw "missing file /x/.unelos/project.yaml"
        if (mode === "empty") return []
        if (mode === "switch") {
          await new Promise((resolve) => setTimeout(resolve, 900))
          const name = root.split("/").pop()!
          return agents.filter((a) => !a.problem).map((a) => ({ ...a, name: `${name}-${a.name}` }))
        }
        return structuredClone(agents)
      }
      case "get_agent": {
        const { id } = args as { id: string }
        if (!Object.hasOwn(agentDocuments, id)) throw `Agent ${id} could not be loaded`
        return structuredClone(agentDocuments[id])
      }
      case "save_agent": {
        const { originalId, agent } = args as { originalId: string | null; agent: AgentDocument }
        if (mode === "error") throw "Project is read-only"
        if (!/^[A-Za-z0-9_-]{1,128}$/.test(agent.id)) throw "Name must contain 1–128 ASCII letters, digits, underscores or hyphens"
        if (originalId === null && agents.some((a) => a.name === agent.id)) throw `Agent ${agent.id} already exists`
        agentDocuments[agent.id] = structuredClone(agent)
        const summary = { name: agent.id, model: agent.model, skills: agent.skills, mcps: agent.mcps, problem: null }
        const index = agents.findIndex((a) => a.name === originalId)
        if (index === -1) agents.push(summary)
        else agents[index] = summary
        return structuredClone(agent)
      }
      case "agent_model_options":
        return {
          aliases: Object.keys(modelMappings),
          harnesses: [
            { id: "opencode", name: "OpenCode", models: Object.entries(harnessModels).flatMap(([provider, models]) => models.map((model) => `${provider}/${model}`)), error: null },
            { id: "codex", name: "Codex", models: ["gpt-5", "gpt-5-mini"], error: null },
            { id: "claude", name: "Claude Code", models: [], error: "Model listing unavailable" },
          ],
        }
      case "list_skills": {
        const root = (args as { root: string }).root
        if (!state.recents.some((r) => r.path === root && r.available)) throw "Project is not approved"
        if (mode === "error") throw "missing file /x/.unelos/project.yaml"
        if (mode === "empty") return []
        return structuredClone(skills)
      }
      case "get_skill":
        return skillDoc((args as { id: string }).id)
      case "save_skill": {
        const { id, name, description, body } = args as { id: string } & SkillDocument & { description: string }
        const s = skills.find((s) => s.name === id)
        if (!s) throw `Skill ${id} not found`
        if (!name || name.trim() !== name || name === "." || name === ".." || /[/\\:\0]/.test(name)) throw "Invalid Skill ID"
        if (name !== id && skills.some((s) => s.name === name)) throw `Skill ${name} already exists`
        const updatedReferences = name === id ? [] : agents.filter((a) => a.skills.includes(id)).map((a) => `.unelos/agents/${a.name}.yaml`)
        for (const a of agents) a.skills = a.skills.map((k) => (k === id ? name : k))
        delete bodies[id]
        bodies[name] = body
        Object.assign(s, { name, description: description || null })
        skills.sort((a, b) => a.name.localeCompare(b.name))
        return { skill: skillDoc(name), updatedReferences }
      }
      case "list_mcps":
      case "list_global_mcps":
        return (cmd === "list_global_mcps" ? globalMcps : mcps).map((m) => ({
          name: m.id,
          kind: m.type,
          target: m.type === "local" ? m.command.join(" ") : m.url,
          problem: null,
        }))
      case "get_mcp":
      case "get_global_mcp": {
        const store = cmd === "get_global_mcp" ? globalMcps : mcps
        const id = (args as { id: string }).id
        const m = store.find((m) => m.id === id)
        if (!m) throw `MCP ${id} not found`
        return structuredClone(m)
      }
      case "save_mcp":
      case "save_global_mcp": {
        const store = cmd === "save_global_mcp" ? globalMcps : mcps
        const prefix = cmd === "save_global_mcp" ? "global/" : ""
        const { originalId, mcp } = args as { originalId: string | null; mcp: McpDocument }
        if (!mcp.id || mcp.id.trim() !== mcp.id || /[/\\:\0.]/.test(mcp.id)) throw "Invalid MCP ID"
        if (mcp.id !== originalId && store.some((m) => m.id === mcp.id)) throw `MCP ${mcp.id} already exists`
        if (mcp.type === "local" ? !mcp.command.length : !mcp.url) throw "MCP needs a command or url"
        const i = store.findIndex((m) => m.id === originalId)
        if (originalId && i < 0) throw `MCP ${originalId} not found`
        if (i >= 0) store[i] = structuredClone(mcp)
        else store.push(structuredClone(mcp))
        if (originalId && originalId !== mcp.id) {
          const projects = prefix ? [...projectAuthoring.values()] : [{ agents, agentDocuments }]
          for (const project of projects) {
            for (const a of project.agents) a.mcps = a.mcps.map((k) => (k === prefix + originalId ? prefix + mcp.id : k))
            for (const d of Object.values(project.agentDocuments)) d.mcps = d.mcps.map((k) => (k === prefix + originalId ? prefix + mcp.id : k))
          }
        }
        store.sort((a, b) => a.id.localeCompare(b.id))
        return structuredClone(mcp)
      }
      case "remove_mcp":
      case "remove_global_mcp": {
        const { id } = args as { id: string }
        const store = cmd === "remove_global_mcp" ? globalMcps : mcps
        const ref = cmd === "remove_global_mcp" ? `global/${id}` : id
        const projects = cmd === "remove_global_mcp" ? [...projectAuthoring.values()] : [{ agents, agentDocuments }]
        if (projects.some((p) => p.agents.some((a) => a.mcps.includes(ref)) || Object.values(p.agentDocuments).some((a) => a.mcps.includes(ref)))) throw `MCP ${ref} is referenced by an agent`
        const i = store.findIndex((m) => m.id === id)
        if (i < 0) throw `MCP ${id} not found`
        store.splice(i, 1)
        return null
      }
      case "get_project_settings":
        return { autoBuildHarness: projectSettings.get(root) ?? null }
      case "save_project_settings":
        projectSettings.set(root, (args as { settings: { autoBuildHarness: boolean | null } }).settings.autoBuildHarness)
        return null
      case "build_harness_config":
      case "auto_build_harness": {
        if (cmd === "auto_build_harness" && projectSettings.get(root) === false) return null
        await new Promise((resolve) => setTimeout(resolve, 400))
        if (mode === "error") throw "permission denied: opencode.json"
        const installed = harnesses.filter((h) => h.installed).map((h) => h.id)
        return { harnesses: installed, written: installed.length ? [".opencode/agents/engineer.md", "opencode.json"] : [], removed: [], skipped: [] }
      }
      case "list_harnesses":
        return structuredClone(harnesses)
      case "install_harness":
      case "uninstall_harness": {
        await new Promise((resolve) => setTimeout(resolve, 900))
        const h = harnesses.find((h) => h.id === (args as { id: string }).id)
        if (!h?.installable) throw `${h?.name ?? "harness"} has no unelos integration yet`
        if (mode === "error") throw "permission denied: ~/.config/opencode/opencode.json"
        Object.assign(h, cmd === "install_harness" ? { installed: true, updateAvailable: false, integrationVersion: "0.0.1" } : { installed: false, updateAvailable: false, integrationVersion: null })
        return structuredClone(harnesses)
      }
      case "plugin:opener|reveal_item_in_dir":
        return null
    }
    throw `dev mock: unhandled command ${cmd}`
  })
}
