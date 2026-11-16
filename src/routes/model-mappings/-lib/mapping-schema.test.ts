import { expect, test } from "vitest"
import { mappingSchema as mapping } from "./mapping-schema"

test("Mappings validate aliases and model strings, not missing harness mappings", () => {
  for (const claude of ["sonnet", "claude-sonnet-4-5"])
    expect(mapping.safeParse({ alias: "smart", models: { claude: { model: claude } } }).success).toBe(true)
  expect(mapping.safeParse({ alias: "fast.v2_test", models: {} }).success).toBeTruthy()
  expect(mapping.safeParse({
      alias: "fast",
      models: { opencode: { model: "" }, hidden: { model: "provider/model" } },
    }).success).toBeTruthy()
  for (const alias of ["", "Fast", "double..dot", "-edge", "has space"])
    expect(mapping.safeParse({ alias, models: {} }).success, alias).toBe(false)
  expect(mapping.safeParse({ alias: "fast", models: { opencode: { model: "has space" } } })
      .success).toBe(false)
})

test("mapping effort is a separate token, not an inline model variant", () => {
  const parse = (model: string, effort?: string) =>
    mapping.safeParse({ alias: "smart", models: { opencode: { model, effort } } }).success
  expect(parse("p/m", "high")).toBe(true)
  expect(parse("p/m")).toBe(true)
  for (const effort of ["", "a b", "x#y"]) expect(parse("p/m", effort)).toBe(false)
  expect(parse("p/m#x")).toBe(false)
})
