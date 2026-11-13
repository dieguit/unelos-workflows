import { expect, test } from "vitest"
import { mappingSchema as mapping } from "./mapping-schema"

test("Mappings validate aliases and model strings, not missing harness mappings", () => {
  expect(mapping.safeParse({ alias: "fast.v2_test", models: {} }).success).toBeTruthy()
  expect(mapping.safeParse({
      alias: "fast",
      models: { opencode: "", hidden: "provider/model" },
    }).success).toBeTruthy()
  for (const alias of ["", "Fast", "double..dot", "-edge", "has space"])
    expect(mapping.safeParse({ alias, models: {} }).success, alias).toBe(false)
  expect(mapping.safeParse({ alias: "fast", models: { opencode: "has space" } })
      .success).toBe(false)
})
