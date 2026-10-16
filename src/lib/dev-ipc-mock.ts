import { mockIPC } from "@tauri-apps/api/mocks"
import type { ProjectState, WorkflowSummary } from "./projects"

const home = "/home/dieguit"
const day = 86_400_000
const mode = new URLSearchParams(location.search).get("mock")

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

export function installDevIpcMock() {
  mockIPC((cmd, args) => {
    const path = (args as { path?: string })?.path
    switch (cmd) {
      case "project_state":
        return structuredClone(state)
      case "open_project": {
        const entry = state.recents.find((r) => r.path === path)
        if (path?.endsWith("/plain")) throw "plain is not a Unelos project (no .unelos folder)"
        if (entry && !entry.available) throw `${entry.name} couldn’t be opened: the folder no longer exists`
        state.recents = [
          { path: path!, name: path!.split("/").pop()!, openedAt: Date.now(), available: true },
          ...state.recents.filter((r) => r.path !== path),
        ]
        return structuredClone(state)
      }
      case "remove_recent":
        state.recents = state.recents.filter((r) => r.path !== path)
        return structuredClone(state)
      case "list_workflows":
        if (mode === "error") throw "missing file /x/.unelos/project.yaml"
        return structuredClone(workflows)
      case "plugin:dialog|open":
        return `${home}/work/api-gateway`
      case "plugin:opener|reveal_item_in_dir":
        return null
    }
    throw `dev mock: unhandled command ${cmd}`
  })
}
