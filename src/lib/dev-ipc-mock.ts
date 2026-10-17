import { mockIPC } from "@tauri-apps/api/mocks"
import type { AgentSummary, ProjectState, SkillSummary, WorkflowSummary } from "./projects"

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
  { name: "engineer", model: "provider/fast", skills: ["rust", "typescript"], mcps: ["github"], problem: null },
  { name: "reviewer", model: null, skills: [], mcps: [], problem: null },
]

const skills: SkillSummary[] = [
  { name: "broken", description: null, problem: "frontmatter: missing closing ---" },
  { name: "code-review", description: "Review code for correctness and missing tests.", problem: null },
  { name: "planning", description: "Write a concrete implementation plan from the request.", problem: null },
  { name: "rust", description: null, problem: null },
]

export function installDevIpcMock() {
  mockIPC(async (cmd, args) => {
    const path = (args as { path?: string })?.path
    switch (cmd) {
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
      case "list_skills": {
        const root = (args as { root: string }).root
        if (!state.recents.some((r) => r.path === root && r.available)) throw "Project is not approved"
        if (mode === "error") throw "missing file /x/.unelos/project.yaml"
        if (mode === "empty") return []
        return structuredClone(skills)
      }
      case "plugin:opener|reveal_item_in_dir":
        return null
    }
    throw `dev mock: unhandled command ${cmd}`
  })
}
