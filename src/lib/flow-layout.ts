import type { WorkflowStep } from "@/lib/projects"

export type FlowRow = {
  step: WorkflowStep
  start: boolean
  /** Edge to the next row; null when this step does not route there. */
  next: { outcomes: string[] } | null
}
export type FlowLane = {
  from: string
  to: string
  outcome: string | null
  /** Row span, inclusive. */
  top: number
  bottom: number
  lane: number
}

/** Steps stay in file order; adjacent edges are inline, every other edge gets a right-side lane. */
export function flowLayout(steps: WorkflowStep[], start: string) {
  const index = new Map(steps.map((step, i) => [step.id, i]))
  const rows: FlowRow[] = steps.map((step, i) => {
    const toNext = step.routes.filter((r) => r.target === steps[i + 1]?.id)
    return {
      step,
      start: step.id === start,
      next: toNext.length
        ? { outcomes: toNext.flatMap((r) => (r.outcome ? [r.outcome] : [])) }
        : null,
    }
  })
  const lanes: FlowLane[] = steps
    .flatMap((step, i) =>
      step.routes.flatMap((r) => {
        const j = index.get(r.target)
        // Unknown targets are reported as issues, not drawn.
        if (j === undefined || j === i + 1) return []
        return [
          {
            from: step.id,
            to: r.target,
            outcome: r.outcome,
            top: Math.min(i, j),
            bottom: Math.max(i, j),
            lane: 0,
          },
        ]
      })
    )
    // Shorter spans hug the cards; longer ones wrap around them.
    .sort((a, b) => a.bottom - a.top - (b.bottom - b.top) || a.top - b.top)
  const taken: FlowLane[][] = []
  for (const lane of lanes) {
    let k = 0
    while (
      taken[k]?.some((o) => o.top <= lane.bottom && lane.top <= o.bottom)
    )
      k++
    lane.lane = k
    ;(taken[k] ??= []).push(lane)
  }
  return { rows, lanes, laneCount: taken.length }
}
