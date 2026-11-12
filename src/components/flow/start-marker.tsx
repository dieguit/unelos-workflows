import { ArrowDownToLineIcon } from "lucide-react"
import { FlowArrow, FlowLine } from "./flow-connector"

/** Sits above whichever step is the start step; it never reorders the flow. */
export function StartMarker() {
  return (
    <div className="flex flex-col items-center">
      <span className="inline-flex items-center gap-1 rounded-full border bg-background px-2.5 py-0.5 text-xs text-muted-foreground">
        <ArrowDownToLineIcon className="size-3" aria-hidden="true" />
        Start
      </span>
      <FlowLine className="h-2" />
      <FlowArrow />
    </div>
  )
}
