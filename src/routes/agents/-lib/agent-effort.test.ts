import { expect, test } from "vitest"
import type { AgentModelOptions } from "@/lib/projects"
import { aliasEfforts, directEfforts, inheritedLabel } from "./agent-effort"

const options = (): AgentModelOptions => ({
  mappings: { smart: { opencode: { model: "a/x", effort: "high" }, claude: { model: "opus" } } },
  harnesses: [
    { id: "opencode", name: "OpenCode", error: null, models: [{ id: "a/x", efforts: ["none", "low", "high"] }] },
    { id: "claude", name: "Claude Code", error: null, models: [{ id: "opus", efforts: ["low", "medium", "high", "xhigh", "max"] }] },
  ],
})

test("alias efforts intersect known mapped models in first harness order", () => {
  expect(aliasEfforts(options(), "smart")).toEqual(["low", "high"])
  const o = options()
  o.harnesses[0].models[0].efforts = []
  expect(aliasEfforts(o, "smart")).toEqual([])
  o.harnesses[0].error = "boom"
  expect(aliasEfforts(o, "smart")).toEqual(o.harnesses[1].models[0].efforts)
  o.harnesses[0].error = null
  o.harnesses[0].models = []
  expect(aliasEfforts(o, "smart")).toEqual(o.harnesses[1].models[0].efforts)
  o.harnesses[1].models = []
  expect(aliasEfforts(o, "smart")).toBeUndefined()
  expect(aliasEfforts(o, "unknown")).toBeUndefined()
})

test("inherited label includes mapped defaults and harness names", () => {
  expect(inheritedLabel(options(), "smart")).toBe("Mapping default (OpenCode: high · Claude Code: default)")
  expect(inheritedLabel(options(), "unknown")).toBe("Mapping default (none)")
})

test("direct efforts exclude failed discovery and unknown models", () => {
  const o = options()
  expect(directEfforts(o, "claude", "opus")).toEqual(o.harnesses[1].models[0].efforts)
  expect(directEfforts(o, "claude", "unknown")).toBeUndefined()
  o.harnesses[1].error = "boom"
  expect(directEfforts(o, "claude", "opus")).toBeUndefined()
})
