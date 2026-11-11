import { ArrowRightIcon, Undo2Icon } from "lucide-react"

export type LaneGeometry = { fromX: number; fromY: number; toX: number; toY: number; x: number }

/** Right-side lane for every edge that does not go to the next step. */
export function ReturnLane({
  geometry: { fromX, fromY, toX, toY, x },
  outcome,
  target,
}: {
  geometry: LaneGeometry
  outcome: string | null
  target: string
}) {
  const up = toY < fromY
  return (
    <>
      <svg aria-hidden="true" className="pointer-events-none absolute inset-0 size-full overflow-visible">
        <path
          d={`M${fromX} ${fromY}H${x}V${toY}H${toX + 6}`}
          className="fill-none stroke-border-strong"
          strokeLinejoin="round"
        />
        <path d={`M${toX} ${toY}l6 -4.5v9z`} className="fill-border-strong" />
      </svg>
      <span
        data-lane-label
        className="absolute flex -translate-x-1/2 -translate-y-1/2 items-center gap-1 rounded-sm border bg-background px-1.5 py-px text-xs whitespace-nowrap text-muted-foreground"
        style={{ left: x, top: (fromY + toY) / 2 }}
      >
        {up && <Undo2Icon className="size-3" aria-hidden="true" />}
        {outcome && <span className="font-mono">{outcome}</span>}
        <ArrowRightIcon className="size-3" aria-hidden="true" />
        <span className="sr-only">to</span>
        <span className="font-semibold text-foreground">{target}</span>
      </span>
    </>
  )
}
