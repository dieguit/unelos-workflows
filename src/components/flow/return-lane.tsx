import { cn } from "cn"
import { ArrowRightIcon, Undo2Icon } from "lucide-react"
import { useState } from "react"

export type LaneGeometry = { fromX: number; fromY: number; toX: number; toY: number; x: number }

/** Right-side lane for every edge that does not go to the next step. */
export function ReturnLane({
  geometry: { fromX, fromY, toX, toY, x },
  outcome,
  target,
  lit,
}: {
  geometry: LaneGeometry
  outcome: string | null
  target: string
  lit?: boolean
}) {
  const up = toY < fromY
  const [hovered, setHover] = useState(false)
  const hover = hovered || lit
  const hoverProps = { onMouseEnter: () => setHover(true), onMouseLeave: () => setHover(false) }
  const d = `M${fromX} ${fromY}H${x}V${toY}H${toX + 6}`
  return (
    <>
      <svg
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 size-full overflow-visible"
        style={{ zIndex: hover ? 1 : 0 }}
      >
        <path d={d} className={cn("fill-none", hover ? "stroke-primary" : "stroke-border-strong")} strokeLinejoin="round" strokeWidth={hover ? 2 : 1} />
        <path d={`M${toX} ${toY}l6 -4.5v9z`} className={hover ? "fill-primary" : "fill-border-strong"} />
        <path d={d} className="fill-none stroke-transparent" strokeWidth={12} style={{ pointerEvents: "stroke" }} {...hoverProps} />
      </svg>
      <span
        data-lane-label
        className={cn(
          "absolute z-[2] flex -translate-x-1/2 -translate-y-1/2 items-center gap-1 rounded-sm border bg-background px-1.5 py-px text-xs whitespace-nowrap text-muted-foreground",
          hover && "z-[3] border-primary bg-linear-to-r from-primary-muted to-primary-muted text-primary",
        )}
        style={{ left: x, top: (fromY + toY) / 2 }}
        {...hoverProps}
      >
        {up && <Undo2Icon className="size-3" aria-hidden="true" />}
        {outcome && <span className="font-mono">{outcome}</span>}
        <ArrowRightIcon className="size-3" aria-hidden="true" />
        <span className="sr-only">to</span>
        <span className={cn("font-semibold", hover ? "text-primary" : "text-foreground")}>{target}</span>
      </span>
    </>
  )
}
