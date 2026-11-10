import assert from "node:assert/strict"
import { after, before, test } from "node:test"
import { resolve } from "node:path"
import { createServer } from "vite"

let server, agent, skill, mapping
before(async () => {
  server = await createServer({
    configFile: false,
    resolve: { alias: { "@": resolve("src") } },
    server: { middlewareMode: true, hmr: false },
  })
  agent = (
    await server.ssrLoadModule("/src/routes/agents/-lib/agent-schema.ts")
  ).agentSchema
  skill = (
    await server.ssrLoadModule("/src/routes/skills/-lib/skill-schema.ts")
  ).skillSchema
  mapping = (
    await server.ssrLoadModule(
      "/src/routes/model-mappings/-lib/mapping-schema.ts"
    )
  ).mappingSchema
})
after(async () => {
  await server.close()
})

test("Agent IDs mirror core safe_id, not lowercase-only aliases", () => {
  const draft = {
    instructions: "",
    skills: [],
    mcps: ["existing"],
    tools: ["kept"],
    model: null,
  }
  for (const id of ["Reviewer_2", "a".repeat(128), "-_"])
    assert.ok(agent.safeParse({ ...draft, id }).success, id)
  for (const id of ["", "a".repeat(129), "has space", "a/b", "é", "a.b"])
    assert.equal(agent.safeParse({ ...draft, id }).success, false, id)
  assert.ok(
    agent.safeParse({ ...draft, id: "reviewer", model: "smart" }).success
  )
  assert.ok(
    agent.safeParse({
      ...draft,
      id: "reviewer",
      model: { harness: "codex", model: "gpt-5" },
    }).success
  )
})

test("Skill names retain their distinct core safety rule; empty Markdown is valid", () => {
  for (const name of ["Rust Guide", "a.b", "Name_2", "é", "a".repeat(129)])
    assert.ok(
      skill.safeParse({ name, description: "", body: "" }).success,
      name
    )
  for (const name of [
    "",
    " leading",
    "trailing ",
    ".",
    "..",
    "a/b",
    "a\\b",
    "a:b",
    "a\0b",
  ])
    assert.equal(
      skill.safeParse({ name, description: "", body: "" }).success,
      false,
      name
    )
})

test("Mappings validate aliases and model strings, not missing harness mappings", () => {
  assert.ok(mapping.safeParse({ alias: "fast.v2_test", models: {} }).success)
  assert.ok(
    mapping.safeParse({
      alias: "fast",
      models: { opencode: "", hidden: "provider/model" },
    }).success
  )
  for (const alias of ["", "Fast", "double..dot", "-edge", "has space"])
    assert.equal(mapping.safeParse({ alias, models: {} }).success, false, alias)
  assert.equal(
    mapping.safeParse({ alias: "fast", models: { opencode: "has space" } })
      .success,
    false
  )
})
