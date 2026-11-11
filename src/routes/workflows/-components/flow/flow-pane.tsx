import { Fragment, useLayoutEffect, useMemo, useRef, useState } from "react"
import type { WorkflowStep } from "@/lib/projects"
import { flowLayout } from "../../-lib/flow-layout"
import { humanize } from "../../-lib/step-edit"
import { FlowConnector } from "./flow-connector"
import { ParallelCard } from "./parallel-card"
import { ReturnLane, type LaneGeometry } from "./return-lane"
import { StartMarker } from "./start-marker"
import { StepCard } from "./step-card"

const LANE_GAP = 32
const LANE_OFFSET = 28
const LABEL_CLEARANCE = 12

type Box = { top: number; bottom: number; right: number }

export function FlowPane({
  steps,
  start,
  issueCounts,
  highlight,
  selected,
  onSelect,
  onAdd,
}: {
  steps: WorkflowStep[]
  start: string
  issueCounts: Map<string, number>
  highlight: string | undefined
  selected?: string
  onSelect?: (id: string) => void
  /** `index` is the row of the step the new one follows. */
  onAdd?: (index: number, kind: "task" | "human-gate") => void
}) {
  const { rows, lanes, laneCount } = useMemo(() => flowLayout(steps, start), [steps, start])
  const column = useRef<HTMLDivElement>(null)
  const [boxes, setBoxes] = useState<Box[]>([])
  const [labelWidths, setLabelWidths] = useState<number[]>([])
  useLayoutEffect(() => {
    const el = column.current
    if (!el) return
    const measure = () => {
      const base = el.getBoundingClientRect()
      const next = [...el.querySelectorAll("[data-flow-card]")].map((card) => {
        const r = card.getBoundingClientRect()
        return { top: r.top - base.top, bottom: r.bottom - base.top, right: r.right - base.left }
      })
      setBoxes((prev) => (JSON.stringify(prev) === JSON.stringify(next) ? prev : next))
      const widths = [...el.querySelectorAll<HTMLElement>("[data-lane-label]")].map((l) => l.offsetWidth)
      setLabelWidths((prev) => (JSON.stringify(prev) === JSON.stringify(widths) ? prev : widths))
    }
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(el)
    return () => observer.disconnect()
  }) // every render: badges and fonts can resize cards without resizing the column
  const geometry = useMemo(() => {
    if (boxes.length !== rows.length) return []
    const laneX: number[] = []
    for (let k = 0; k < laneCount; k++) {
      // Labels sit centred on their lane, so the lane keeps half a label clear of the cards it passes.
      const clear = Math.max(
        0,
        ...lanes.flatMap((l, i) =>
          l.lane === k
            ? boxes
                .slice(l.top, l.bottom + 1)
                .map((b) => b.right + Math.max(LANE_OFFSET, (labelWidths[i] ?? 0) / 2 + LABEL_CLEARANCE))
            : []
        )
      )
      laneX[k] = Math.max(clear, (laneX[k - 1] ?? 0) + LANE_GAP)
    }
    const index = new Map(rows.map((r, i) => [r.step.id, i]))
    return lanes.map((lane): LaneGeometry => {
      const from = boxes[index.get(lane.from)!]
      const to = boxes[index.get(lane.to)!]
      const mid = (b: Box) => (b.top + b.bottom) / 2
      // Outgoing below centre, incoming above, so a card with both never overlaps.
      return { fromX: from.right, fromY: mid(from) + 6, toX: to.right, toY: mid(to) - 6, x: laneX[lane.lane] }
    })
  }, [boxes, labelWidths, rows, lanes, laneCount])
  const contentRight = Math.max(0, ...boxes.map((b) => b.right))
  const laneRoom = Math.max(0, ...geometry.map((g, i) => g.x + (labelWidths[i] ?? 0) / 2 + 16 - contentRight))
  return (
    <div className="min-h-0 flex-1 overflow-auto px-8 py-10">
      <div
        ref={column}
        className="relative mx-auto flex w-fit flex-col items-center"
        style={{ paddingRight: laneRoom }}
      >
        {rows.map((row, i) => (
          <Fragment key={row.step.id}>
            {row.start && <StartMarker />}
            <div data-flow-card={row.step.id}>
              {row.step.kind === "parallel" ? (
                <ParallelCard step={row.step} issues={issueCounts.get(row.step.id) ?? 0} highlight={highlight === row.step.id} />
              ) : (
                <StepCard
                  step={row.step}
                  issues={issueCounts.get(row.step.id) ?? 0}
                  highlight={highlight === row.step.id}
                  selected={selected === row.step.id}
                  onSelect={row.step.kind === "end" ? undefined : onSelect}
                />
              )}
            </div>
            {i < rows.length - 1 && (
              <FlowConnector
                outcomes={row.next?.outcomes ?? null}
                onAdd={onAdd && ((kind) => onAdd(i, kind))}
              />
            )}
          </Fragment>
        ))}
        {geometry.map((g, i) => (
          <ReturnLane key={i} geometry={g} outcome={lanes[i].outcome} target={humanize(lanes[i].to)} />
        ))}
      </div>
    </div>
  )
}
