import { expect, test } from "vitest"
import { groupReferences, usedByFor, usedByLabel } from "@/lib/references"

const wf = (id: string) => ({ kind: "workflow" as const, id, name: id })
const ag = (id: string) => ({ kind: "agent" as const, id, name: id })

test("usedByLabel counts by kind", () => {
  expect(usedByLabel(null)).toBe("—")
  expect(usedByLabel([])).toBe("—")
  expect(usedByLabel([wf("a")])).toBe("1 workflow")
  expect(usedByLabel([ag("x"), wf("a"), wf("b")])).toBe("2 workflows, 1 agent")
})

test("groupReferences orders workflow, agent, task", () => {
  expect(groupReferences([ag("x"), wf("a")]).map((g) => g.kind)).toEqual(["workflow", "agent"])
})

test("usedByFor reads global/ keys and names the project only for global pages", () => {
  const map = { s: [wf("a")], "global/s": [ag("x")] }
  expect(usedByFor(map).of("s")).toEqual([wf("a")])
  expect(usedByFor(map, { global: true, project: "P" }).of("s")).toEqual([ag("x")])
  expect(usedByFor(map, { global: true, project: "P" }).project).toBe("P")
  expect(usedByFor(map).of("none")).toEqual([])
  expect(usedByFor(null).of("s")).toBe(null)
})
