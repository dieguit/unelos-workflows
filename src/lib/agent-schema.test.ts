import { expect, test } from "vitest"
import { agentSchema as agent } from "./agent-schema"

test("Agent IDs mirror core safe_id, not lowercase-only aliases", () => {
  const draft = {
    instructions: "",
    skills: [],
    mcps: ["existing"],
    tools: ["kept"],
    model: null,
    effort: null,
  }
  for (const id of ["Reviewer_2", "a".repeat(128), "-_"])
    expect(agent.safeParse({ ...draft, id }).success, id).toBeTruthy()
  for (const id of ["", "a".repeat(129), "has space", "a/b", "é", "a.b"])
    expect(agent.safeParse({ ...draft, id }).success, id).toBe(false)
  expect(agent.safeParse({ ...draft, id: "reviewer", model: "smart" }).success).toBeTruthy()
  expect(agent.safeParse({
      ...draft,
      id: "reviewer",
      model: { harness: "codex", model: "gpt-5" },
    }).success).toBeTruthy()
})

test("agent effort survives schema parsing as null or an override", () => {
  const draft = { id: "reviewer", instructions: "", skills: [], mcps: [], tools: [], model: "smart" }
  for (const effort of [null, "high"])
    expect(agent.parse({ ...draft, effort })).toHaveProperty("effort", effort)
})
