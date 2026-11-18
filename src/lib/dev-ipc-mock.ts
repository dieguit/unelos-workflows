import { mockIPC } from "@tauri-apps/api/mocks"
import type { Harness } from "./harnesses"
import type { MappedModel, ModelMappings } from "./model-mappings"
import type { ProjectReferences, Reference, McpDocument, TaskDocument, AgentDocument, AgentSummary, ProjectState, SkillDocument, SkillSummary, WorkflowDocument, WorkflowDraft, WorkflowStep, WorkflowIssue, WorkflowSummary } from "./projects"

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
        { global: false, calls: [], name: "Implement feature", file: "implement-feature.yaml", description: "Plan, approve, implement and review a feature with a correction loop.", steps: 5, problem: null },
        { global: false, calls: [], name: "Investigate bug", file: "investigate-bug.yaml", description: "Reproduce the issue, locate the root cause and propose a fix with evidence.", steps: 4, problem: null },
        { global: false, calls: [], name: "Review change", file: "review-change.yaml", description: "Independent review of a diff with a findings report and human sign-off.", steps: 3, problem: "nodes.review.agent: Agent reference missing" },
        { global: false, calls: [], name: "Refactor module", file: "refactor-module.yaml", description: "Map affected code, refactor in parallel slices and verify behavior is unchanged.", steps: 6, problem: null },
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
        document: { id, name: id, description: summary.description, start: steps[0]?.id ?? "", inputs: [], steps, issues: summary.problem ? [{ message: summary.problem, step: "review" }] : steps.length ? [] : [{ message: "Mock fixture has no step definitions", step: null }] },
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
    skills: [...a.skills], mcps: [...a.mcps], tools: a.name === "engineer" ? ["read-file", "run-command"] : [], model: a.model, effort: null,
  }])
)

const globalAgents: AgentSummary[] = mode === "empty" ? [] : [
  { name: "architect", model: "smart", skills: ["global/tdd"], mcps: [], problem: null },
  { name: "writer", model: null, skills: [], mcps: [], problem: null },
]
const globalAgentDocuments: Record<string, AgentDocument> = Object.fromEntries(globalAgents.map((a) => [a.name, {
  id: a.name, instructions: a.name === "architect" ? "Design the approach before any code is written." : "Write clear, short documentation.",
  skills: [...a.skills], mcps: [...a.mcps], tools: [], model: a.model, effort: null,
}]))

const globalSkills: SkillSummary[] = [{ name: "tdd", description: "Write the failing test first.", problem: null }]

const globalWorkflowStep = (id: string, kind: WorkflowStep["kind"], next: string | null, agent: string | null = null): WorkflowStep => ({
  id, originalId: id, label: id, kind, title: kind === "task" ? id : null, agent, execution: kind === "task" ? "main" : null,
  task: kind === "task" ? { source: "inline", id: null, description: null, instructions: `Perform ${id}`, skills: [], outcomes: [] } : null,
  routes: next ? [{ outcome: null, target: next }] : [], workers: [], inputs: [], outputs: [],
})
const globalWorkflows: { summary: WorkflowSummary; document: WorkflowDocument }[] = mode === "empty" ? [] : [{
  summary: { global: true, calls: [], name: "global/workflow-design", file: "global/workflow-design.yaml", description: "Design a change with a global architect before any project writes code.", steps: 3, problem: null },
  document: {
    id: "workflow-design", name: "workflow-design", description: "Design a change with a global architect before any project writes code.", start: "design", inputs: [],
    steps: [globalWorkflowStep("design", "task", "approve", "global/architect"), globalWorkflowStep("approve", "human-gate", "done"), globalWorkflowStep("done", "end", null)],
    issues: [],
  },
}]
function globalWorkflowFor(id: string) {
  const workflow = globalWorkflows.find((w) => w.document.id === id)
  if (!workflow) throw `Global workflow ${id} could not be loaded`
  return workflow
}
function globalWorkflowIssues(id: string, draft: WorkflowDraft): WorkflowIssue[] {
  globalWorkflowFor(id)
  if (draft.name !== id) {
    if (!/^[A-Za-z0-9_-]{1,128}$/.test(draft.name)) return [{ message: "Name must contain 1–128 ASCII letters, digits, underscores or hyphens", step: null }]
    if (globalWorkflows.some((w) => w.document.id === draft.name)) return [{ message: `Workflow ${draft.name} already exists`, step: null }]
  }
  if (!draft.steps.some((step) => step.id === draft.start)) return [{ message: `start: unknown target ${JSON.stringify(draft.start)}`, step: null }]
  const globalOnly = (step: WorkflowStep) => [
    ...(step.kind === "task" ? [{ agent: step.agent, task: step.task }] : step.workers),
  ].flatMap(({ agent, task }) => [
    ...(!agent?.trim() ? [{ message: "Pick an agent", step: step.id }] : !agent.startsWith("global/") ? [{ message: `Global workflows can only use global agents (${agent})`, step: step.id }] : []),
    ...(task?.source === "named" && task.id && !task.id.startsWith("global/") ? [{ message: `Global workflows can only use global tasks (${task.id})`, step: step.id }] : []),
  ])
  return [
    ...draft.steps.filter((s) => s.kind === "task" || s.kind === "parallel").flatMap(globalOnly),
    ...draft.steps.filter((s) => s.kind === "workflow" && s.call?.workflow && !s.call.workflow.startsWith("global/")).map((s) => ({ message: `Global workflows can only call global workflows (${s.call!.workflow})`, step: s.id })),
  ]
}

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
  id, name, installable: id === "opencode" || id === "claude", detected: version !== null, installed: false, updateAvailable: false,
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
  smart: { opencode: { model: "anthropic/claude-sonnet-4-5", effort: "high" }, claude: { model: "opus", effort: "max" } },
  balanced: {},
  fast: { opencode: { model: "openai/gpt-5-mini" } },
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
const claudeModels = ["fable", "opus", "sonnet", "haiku"].map((id) => ({
  id, efforts: ["low", "medium", "high", "xhigh", "max"],
}))
const opencodeModels = Object.entries(harnessModels).flatMap(([provider, models]) =>
  models.map((model) => ({
    id: `${provider}/${model}`,
    efforts: provider === "openai" ? ["none", "low", "medium", "high", "xhigh", "max"]
      : provider === "anthropic" ? ["low", "medium", "high", "xhigh", "max"] : [],
  }))
).sort((a, b) => a.id.localeCompare(b.id))

// Mock modes: import-clean | import-mcp (default) | import-conflicts | import-many | import-source-error | import-commit-fail | import-commit-conflicts
const importMode = mode?.startsWith("import-") ? mode.slice(7) : "mcp"
const importSessions = new Map<string, { source: string; commits: number }>()
const importCancelled = new Set<string>()
const NAME_FORMAT = /^[a-z0-9]+(-[a-z0-9]+)*$/
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

type MockPlan = {
  source: string
  workflow: { id: string; description?: string }
  items: { kind: string; id: string }[]
  reused: string[]
  mcps: { id: string; command?: string; url?: string }[]
  warnings: { kind: string; message: string }[]
  conflicts: { key: string; kind: string; id: string; suggested: string; reason: string; causedBy?: string; error?: string }[]
}

function importPlan(root: string, source: string, renames: Record<string, string>, extraTaken: string[] = []): MockPlan {
  const withMcps = importMode === "mcp" || importMode === "commit-fail" || importMode === "commit-conflicts"
  const items = [
    { kind: "workflow", id: importMode === "conflicts" ? "investigate-bug" : "research" },
    { kind: "agent", id: importMode === "conflicts" ? "engineer" : "researcher" },
    { kind: "task", id: "review-change" },
    { kind: "skill", id: importMode === "conflicts" ? "code-review" : "web-search" },
    ...(withMcps ? [{ kind: "mcp", id: "github" }, { kind: "mcp", id: "search" }] : []),
  ]
  if (importMode === "many") for (let i = 1; i <= 12; i++) items.push({ kind: "skill", id: `lint-${i}` })
  const authoring = authoringFor(root)
  const existing: Record<string, string[]> = {
    workflow: workflowsFor(root).map((w) => w.document.id),
    agent: authoring.agents.map((a) => a.name),
    skill: [...authoring.skills.map((s) => s.name), ...extraTaken, ...(importMode === "many" ? Array.from({ length: 12 }, (_, i) => `lint-${i + 1}`) : [])],
    task: [],
    mcp: [],
  }
  const base: [string, string, string, string?][] = items
    .filter((i) => existing[i.kind].includes(i.id))
    .map((i) => [`${i.kind}:${i.id}`, i.kind, i.id])
  const skillRenamed = importMode === "conflicts" && renames["skill:code-review"] !== "code-review"
  const cascaded: [string, string, string, string?][] = skillRenamed ? [["task:review-change", "task", "review-change", "skill:code-review"]] : []
  const rows = [...base, ...cascaded]
  const incoming = (kind: string) => items.filter((i) => i.kind === kind).map((i) => i.id)
  const taken = new Set<string>()
  const suggested = (kind: string, id: string) => {
    let n = 2
    while (existing[kind].includes(`${id}-${n}`) || incoming(kind).includes(`${id}-${n}`) || taken.has(`${kind}:${id}-${n}`)) n++
    taken.add(`${kind}:${id}-${n}`)
    return `${id}-${n}`
  }
  const prefilled = rows.map(([key, kind, id]) => [key, suggested(kind, id)] as const)
  const finalName = (key: string, fallback: string) => renames[key] ?? fallback
  const names = rows.map(([key], i) => finalName(key, prefilled[i][1]))
  const conflicts = rows.map(([key, kind, id, causedBy], i) => {
    const name = names[i]
    const fixed = incoming(kind).filter((x) => !rows.some((r) => r[0] === `${kind}:${x}`))
    const error = !name
      ? "Enter a name"
      : !NAME_FORMAT.test(name)
        ? "Use lowercase letters, numbers and dashes"
        : existing[kind].includes(name)
          ? `A ${kind} named “${name}” already exists`
          : fixed.includes(name) || names.some((n, j) => j !== i && n === name && rows[j][1] === kind)
            ? `Two items can’t both be named “${name}”`
            : undefined
    return { key, kind, id, suggested: prefilled[i][1], reason: causedBy ? "referencesRenamed" : "exists", ...(causedBy ? { causedBy } : {}), ...(error ? { error } : {}) }
  })
  return {
    source,
    workflow: { id: items[0].id, description: "Research a topic, triage findings and produce a sourced summary." },
    items,
    reused: importMode === "clean" ? ["skill:planning"] : importMode === "conflicts" ? [] : ["skill:planning", "agent:reviewer"],
    mcps: withMcps ? [{ id: "github", url: "https://api.githubcopilot.com/mcp/" }, { id: "search", command: "npx -y @modelcontextprotocol/server-brave-search@0.6.2" }, { id: "fetch", command: "uvx mcp-server-fetch==2025.4.7 --ignore-robots-txt --user-agent unelos-research" }] : [],
    warnings: importMode === "clean" || importMode === "conflicts"
      ? []
      : [
          { kind: "missingGlobal", message: "Uses global skill `global/tdd`, which isn’t installed on this machine" },
          { kind: "unresolvedModel", message: "Agent `reviewer` uses model `claude-x`, which isn’t mapped" },
        ],
    conflicts,
  }
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
      case "project_references": {
        const result: ProjectReferences = { agents: {}, tasks: {}, skills: {}, mcps: {} }
        const add = (map: Record<string, Reference[]>, id: string, reference: Reference) => {
          if (!id) return
          const entries = map[id] ??= []
          if (!entries.some((entry) => entry.kind === reference.kind && entry.id === reference.id)) entries.push(reference)
        }
        for (const agent of agents.filter((agent) => !agent.problem)) {
          const reference: Reference = { kind: "agent", id: agent.name, name: agent.name }
          agent.skills.forEach((id) => add(result.skills, id, reference))
          agent.mcps.forEach((id) => add(result.mcps, id, reference))
        }
        for (const task of Object.values(tasks)) {
          task.skills.forEach((id) => add(result.skills, id, { kind: "task", id: task.id, name: task.id }))
        }
        for (const { document, summary } of workflowsFor(root)) {
          if (summary.problem) continue
          const reference: Reference = { kind: "workflow", id: document.id, name: document.name }
          for (const step of document.steps) {
            const selections = step.kind === "task" ? [{ agent: step.agent, task: step.task }] : step.workers
            for (const { agent, task } of selections) {
              if (agent) add(result.agents, agent, reference)
              if (task?.source === "named" && task.id) add(result.tasks, task.id, reference)
              if (task?.source === "inline") task.skills.forEach((id) => add(result.skills, id, reference))
            }
          }
        }
        Object.values(result).forEach((map) => Object.values(map).forEach((entries) => entries.sort((a, b) => a.name.localeCompare(b.name))))
        return result
      }
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
        const { original, alias, models } = args as { original: string | null; alias: string; models: Record<string, MappedModel> }
        if (!/^[a-z0-9]+([-_.][a-z0-9]+)*$/.test(alias)) throw "Alias must be lowercase letters and digits separated by -, _ or ."
        const filtered = Object.fromEntries(Object.entries(models)
          .filter(([, entry]) => entry.model !== "")
          .map(([id, entry]) => [id, { model: entry.model, ...(entry.effort ? { effort: entry.effort } : {}) }]))
        if (Object.values(filtered).some((entry) => /[\s#]/u.test(entry.model))) throw "Model must not contain whitespace or #"
        if (Object.values(filtered).some((entry) => entry.effort && /[\s#]/u.test(entry.effort))) throw "Invalid effort"
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
        if ((args as { id: string }).id === "claude") return structuredClone(claudeModels)
        if ((args as { id: string }).id !== "opencode") throw "Harness models unavailable"
        return structuredClone(opencodeModels)
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
      case "uninstall_project": {
        const { root } = args as { root: string }
        if (!state.recents.some((r) => r.path === root && r.available))
          throw "Project is not approved; select it using Open folder…"
        state.recents = state.recents.filter((r) => r.path !== root)
        return structuredClone(state)
      }
      case "list_workflows": {
        const root = (args as { root: string }).root
        if (mode === "switch") await new Promise((resolve) => setTimeout(resolve, 900))
        const own = workflowsFor(root).map((w) => w.summary)
        return structuredClone(mode === "error" ? own : [...own, ...globalWorkflows.map((w) => w.summary)])
      }
      case "list_global_workflows":
        return structuredClone(globalWorkflows.map((w) => w.summary))
      case "get_global_workflow":
        return structuredClone(globalWorkflowFor((args as { id: string }).id).document)
      case "create_global_workflow": {
        let id = "new-workflow"
        for (let n = 2; globalWorkflows.some((w) => w.document.id === id); n++) id = `new-workflow-${n}`
        globalWorkflows.push({
          summary: { global: true, calls: [], name: `global/${id}`, file: `global/${id}.yaml`, description: null, steps: 2, problem: null },
          document: { id, name: id, description: null, start: "new-task", inputs: [], steps: [globalWorkflowStep("new-task", "task", "end", ""), globalWorkflowStep("end", "end", null)], issues: [{ message: "Pick an agent", step: "new-task" }] },
        })
        return id
      }
      case "validate_global_workflow": {
        const { id, draft } = args as { id: string; draft: WorkflowDraft }
        return globalWorkflowIssues(id, draft)
      }
      case "save_global_workflow": {
        const { id, draft } = args as { id: string; draft: WorkflowDraft }
        await new Promise((resolve) => setTimeout(resolve, 900))
        if (mode === "save-error") throw "Config folder is read-only (mock save failure)"
        const issues = globalWorkflowIssues(id, draft)
        if (issues.length) throw issues.map((issue) => issue.message).join("\n")
        const workflow = globalWorkflowFor(id)
        Object.assign(workflow.document, structuredClone(draft), { id: draft.name, steps: structuredClone(draft.steps).map((step) => ({ ...step, originalId: step.id })), issues: [] })
        Object.assign(workflow.summary, { name: `global/${draft.name}`, file: `global/${draft.name}.yaml`, description: draft.description, steps: draft.steps.length, problem: null })
        return structuredClone(workflow.document)
      }
      case "duplicate_global_workflow": {
        const { id, newId } = args as { id: string; newId: string }
        const workflow = globalWorkflowFor(id)
        if (!/^[A-Za-z0-9_-]{1,128}$/.test(newId)) throw "Name must contain 1–128 ASCII letters, digits, underscores or hyphens"
        if (globalWorkflows.some((w) => w.document.id === newId)) throw "Already exists"
        globalWorkflows.push({ summary: { ...structuredClone(workflow.summary), name: `global/${newId}`, file: `global/${newId}.yaml` }, document: { ...structuredClone(workflow.document), name: newId, id: newId } })
        return null
      }
      case "remove_global_workflow": {
        const { id } = args as { id: string }
        const index = globalWorkflows.findIndex((w) => w.document.id === id)
        if (index === -1) throw `Global workflow ${id} not found`
        globalWorkflows.splice(index, 1)
        return null
      }
      case "list_global_agents":
        return structuredClone(globalAgents)
      case "get_global_agent": {
        const { id } = args as { id: string }
        if (!Object.hasOwn(globalAgentDocuments, id)) throw `Global agent ${id} could not be loaded`
        return structuredClone(globalAgentDocuments[id])
      }
      case "save_global_agent": {
        const { originalId, agent } = args as { originalId: string | null; agent: AgentDocument }
        if (mode === "save-error") throw "Config folder is read-only (mock save failure)"
        if (!/^[A-Za-z0-9_-]{1,128}$/.test(agent.id)) throw "Name must contain 1–128 ASCII letters, digits, underscores or hyphens"
        if (originalId === null && globalAgents.some((a) => a.name === agent.id)) throw `Agent ${agent.id} already exists`
        globalAgentDocuments[agent.id] = structuredClone(agent)
        const summary = { name: agent.id, model: agent.model, skills: agent.skills, mcps: agent.mcps, problem: null }
        const index = globalAgents.findIndex((a) => a.name === originalId)
        if (index === -1) globalAgents.push(summary)
        else globalAgents[index] = summary
        return structuredClone(agent)
      }
      case "duplicate_global_agent": {
        const { id, newId } = args as { id: string; newId: string }
        const agent = globalAgents.find((a) => a.name === id)
        if (!agent) throw `Global agent ${id} not found`
        if (!/^[A-Za-z0-9_-]{1,128}$/.test(newId)) throw "Name must contain 1–128 ASCII letters, digits, underscores or hyphens"
        if (globalAgents.some((a) => a.name === newId)) throw "Already exists"
        globalAgents.push({ ...structuredClone(agent), name: newId })
        globalAgentDocuments[newId] = { ...structuredClone(globalAgentDocuments[id]), id: newId }
        return null
      }
      case "remove_global_agent": {
        const { id } = args as { id: string }
        const index = globalAgents.findIndex((a) => a.name === id)
        if (index === -1) throw `Global agent ${id} not found`
        if (globalWorkflows.some((w) => w.document.steps.some((s) => s.agent === `global/${id}`))) throw "Cannot remove Agent: referenced by a global workflow (mock fixture)"
        globalAgents.splice(index, 1)
        delete globalAgentDocuments[id]
        return null
      }
      case "list_global_skills":
        return structuredClone(globalSkills)
      case "get_global_skill": {
        const { id } = args as { id: string }
        const skill = globalSkills.find((s) => s.name === id)
        if (!skill) throw `Skill ${id} not found`
        return { name: id, description: skill.description, body: `# ${id}\n` }
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
          id, name: id, description: null, start: "new-task", inputs: [],
          steps: [
            { id: "new-task", originalId: "new-task", label: "new-task", kind: "task", title: "new-task", agent: "", execution: "main", task: { source: "inline", id: null, description: null, instructions: "Change me!", skills: [], outcomes: [] }, routes: [{ outcome: null, target: "end" }], workers: [], inputs: [], outputs: [] },
            { id: "end", originalId: "end", label: "end", kind: "end", title: null, agent: null, execution: null, task: null, routes: [], workers: [], inputs: [], outputs: [] },
          ],
          issues: [{ message: "Pick an agent", step: "new-task" }],
        }
        workflows.push({ summary: { global: false, calls: [], name: id, file: `${id}.yaml`, description: null, steps: 2, problem: null }, document })
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
          mappings: structuredClone(modelMappings),
          harnesses: [
            { id: "opencode", name: "OpenCode", models: structuredClone(opencodeModels), error: null },
            { id: "claude", name: "Claude Code", models: structuredClone(claudeModels), error: null },
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
      case "import_prepare": {
        const { id, source } = args as { id: string; source: string }
        importCancelled.delete(id)
        await sleep(/^[a-z]+:\/\//.test(source) ? 1800 : 500)
        if (importCancelled.has(id)) throw "Cancelled"
        if (importMode === "source-error") {
          if (/^[a-z]+:\/\//.test(source)) throw "Couldn’t clone repository: authentication failed\nCheck that git can reach this repository with your credentials, or clone it yourself and choose the local folder."
          throw "This folder must contain exactly one workflow (found 3)\nFound research, triage and release-notes. Point to the subfolder of the one you want, e.g. ~/projects/agent-workflows/research."
        }
        importSessions.set(id, { source, commits: 0 })
        return importPlan(root, source, {})
      }
      case "import_plan": {
        const { id, renames } = args as { id: string; renames: Record<string, string> }
        const session = importSessions.get(id)
        if (!session) throw "Import session expired"
        return importPlan(root, session.source, renames)
      }
      case "import_commit": {
        const { id, renames } = args as { id: string; renames: Record<string, string> }
        const session = importSessions.get(id)
        if (!session) throw "Import session expired"
        await sleep(900)
        session.commits++
        if (importMode === "commit-fail" && session.commits === 1) throw "Couldn’t write skills/code-review-2/SKILL.md: no space left on device (os error 28)\nFree up space, then retry. Your names and choices are kept."
        if (importMode === "commit-conflicts" && session.commits === 1) return { status: "conflicts", plan: importPlan(root, session.source, renames, ["web-search"]) }
        const plan = importPlan(root, session.source, renames)
        if (plan.conflicts.some((c) => c.error)) return { status: "conflicts", plan }
        const key = `workflow:${plan.workflow.id}`
        const name = renames[key] ?? plan.conflicts.find((c) => c.key === key)?.suggested ?? plan.workflow.id
        workflowsFor(root).push({
          summary: { global: false, calls: [], name, file: `${name}.yaml`, description: plan.workflow.description ?? null, steps: 2, problem: null },
          document: {
            id: name, name, description: plan.workflow.description ?? null, start: "end", inputs: [],
            steps: [{ id: "end", originalId: "end", label: "end", kind: "end", title: null, agent: null, execution: null, task: null, routes: [], workers: [], inputs: [], outputs: [] }],
            issues: [],
          },
        })
        importSessions.delete(id)
        return { status: "imported", workflow: name, warningCount: plan.warnings.length }
      }
      case "import_cancel": {
        const { id } = args as { id: string }
        importCancelled.add(id)
        importSessions.delete(id)
        return null
      }
      case "plugin:dialog|open":
        return `${home}/projects/agent-workflows`
      case "plugin:opener|reveal_item_in_dir":
        return null
    }
    throw `dev mock: unhandled command ${cmd}`
  })
}
