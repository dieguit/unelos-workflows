import { expect, test } from "vitest"
import { skillSchema as skill } from "./skill-schema"

test("Skill names retain their distinct core safety rule; empty Markdown is valid", () => {
  for (const name of ["Rust Guide", "a.b", "Name_2", "é", "a".repeat(129)])
    expect(skill.safeParse({ name, description: "", body: "" }).success, name).toBeTruthy()
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
    expect(skill.safeParse({ name, description: "", body: "" }).success, name).toBe(false)
})
