import { expect, test } from "vitest"
import { flowLayout } from "@/lib/flow-layout.ts"
import type { WorkflowStep } from "./projects"

const step = (id: string, kind: WorkflowStep["kind"], routes: [string | null, string][] = []): WorkflowStep => ({
  id, label: id, kind, title: null, agent: null, execution: null,
  routes: routes.map(([outcome, target]) => ({ outcome, target })),
  originalId: id, task: null, workers: [], inputs: [], outputs: [],
})
const feature = [
  step("plan", "task", [[null, "approve-plan"]]),
  step("approve-plan", "human-gate", [["approved", "implement"], ["rejected", "plan"]]),
  step("implement", "parallel", [[null, "review"]]),
  step("review", "task", [["approved", "done"], ["changes_requested", "implement"]]),
  step("done", "end"),
]

test("adjacent edges are inline, labeled only by declared outcomes", () => {
  const { rows } = flowLayout(feature, "plan")
  expect(rows.map((r) => r.next?.outcomes ?? null)).toEqual([[], ["approved"], [], ["approved"], null])
  expect(rows.map((r) => r.start)).toEqual([true, false, false, false, false])
})

test("returns become lanes; overlapping spans get separate lanes", () => {
  const { lanes, laneCount } = flowLayout(feature, "implement")
  expect(lanes.map((l) => [l.outcome, l.from, l.to, l.lane])).toEqual([
    ["rejected", "approve-plan", "plan", 0],
    ["changes_requested", "review", "implement", 0],
  ])
  expect(laneCount).toBe(1)
  const skip = [...feature.slice(0, 4), step("done", "end")]
  skip[0] = step("plan", "task", [["skip", "review"], ["ok", "approve-plan"]])
  const out = flowLayout(skip, "plan")
  expect(out.lanes.map((l) => [l.from, l.to, l.lane])).toEqual([
    ["approve-plan", "plan", 0],
    ["review", "implement", 0],
    ["plan", "review", 1],
  ])
  expect(out.laneCount).toBe(2)
})

test("unknown targets and missing start are ignored", () => {
  const { rows, lanes } = flowLayout([step("a", "task", [[null, "ghost"]])], "nope")
  expect(rows[0].next).toBe(null)
  expect(rows[0].start).toBe(false)
  expect(lanes).toEqual([])
})
