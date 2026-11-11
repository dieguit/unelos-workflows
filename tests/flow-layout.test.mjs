import assert from "node:assert/strict"
import { test } from "node:test"
import { flowLayout } from "../src/routes/workflows/-lib/flow-layout.ts"

const step = (id, kind, routes = []) => ({
  id, label: id, kind, title: null, agent: null, execution: null,
  routes: routes.map(([outcome, target]) => ({ outcome, target })),
  workers: [], inputs: [], outputs: [],
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
  assert.deepEqual(rows.map((r) => r.next?.outcomes ?? null), [[], ["approved"], [], ["approved"], null])
  assert.deepEqual(rows.map((r) => r.start), [true, false, false, false, false])
})

test("returns become lanes; overlapping spans get separate lanes", () => {
  const { lanes, laneCount } = flowLayout(feature, "implement")
  assert.deepEqual(lanes.map((l) => [l.outcome, l.from, l.to, l.lane]), [
    ["rejected", "approve-plan", "plan", 0],
    ["changes_requested", "review", "implement", 0],
  ])
  assert.equal(laneCount, 1)
  const skip = [...feature.slice(0, 4), step("done", "end")]
  skip[0] = step("plan", "task", [["skip", "review"], ["ok", "approve-plan"]])
  const out = flowLayout(skip, "plan")
  assert.deepEqual(out.lanes.map((l) => [l.from, l.to, l.lane]), [
    ["approve-plan", "plan", 0],
    ["review", "implement", 0],
    ["plan", "review", 1],
  ])
  assert.equal(out.laneCount, 2)
})

test("unknown targets and missing start are ignored", () => {
  const { rows, lanes } = flowLayout([step("a", "task", [[null, "ghost"]])], "nope")
  assert.equal(rows[0].next, null)
  assert.equal(rows[0].start, false)
  assert.deepEqual(lanes, [])
})
