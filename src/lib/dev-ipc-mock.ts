import { mockIPC } from "@tauri-apps/api/mocks"
import type { Harness } from "./harnesses"
import type { ModelMappings } from "./model-mappings"
import type { AgentDocument, AgentSummary, ProjectState, SkillDocument, SkillSummary, WorkflowSummary } from "./projects"

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

const bodies: Record<string, string> = {
  "code-review": "# Code review\n\nInspect the actual changes, not a summary of them.\n\n## Correctness\n- Read the diff against the plan before judging it.\n- Check edge cases, error handling and regressions.\n\n## Tests\n- Confirm changed behavior is covered by tests.\n- Flag missing or weak tests with file and line.\n\nReport findings, then select an outcome.\n",
  planning: "# Planning\n\nWrite a concrete, ordered plan before changing code.\n",
  rust: "# Rust\n\nFollow existing Rust conventions in this project.\n",
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

const skillDoc = (id: string): SkillDocument => {
  const s = skills.find((s) => s.name === id)
  if (!s) throw `Skill ${id} not found`
  if (s.problem) throw `${id}/SKILL.md: ${s.problem}`
  return { name: s.name, description: s.description, body: bodies[id] ?? "" }
}

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
    switch (cmd) {
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
        if (!state.recents.some((r) => r.path === root && r.available)) throw "Project is not approved"
        if (mode === "error") throw "missing file /x/.unelos/project.yaml"
        if (mode === "switch") {
          await new Promise((resolve) => setTimeout(resolve, 900))
          const name = root.split("/").pop()!
          return workflows.map((w) => ({ ...w, name: `${name}: ${w.name}`, file: `${name}-${w.file}` }))
        }
        return structuredClone(workflows)
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
