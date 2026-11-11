import type { ReactNode } from "react"
import { stepIconColors, stepIcons, stepKindLabels } from "./step-icon"

/** Toolbar above the flow: a start step picker slot and a static legend. */
export function FlowToolbar({ startPicker }: { startPicker: ReactNode }) {
  return (
    <div className="flex h-11 shrink-0 items-center gap-4 overflow-x-auto border-b px-4 text-sm">
      {startPicker}
      <div className="h-5 border-l" />
      <ul aria-label="Legend" className="flex items-center gap-4 text-muted-foreground">
        {(["task", "parallel", "human-gate", "end"] as const).map((kind) => {
          const Icon = stepIcons[kind]
          return (
            <li key={kind} className="flex shrink-0 items-center gap-1.5">
              <span className="flex size-5 items-center justify-center rounded-sm bg-muted">
                <Icon className={`size-3 ${stepIconColors[kind]}`} aria-hidden="true" />
              </span>
              {stepKindLabels[kind]}
            </li>
          )
        })}
      </ul>
    </div>
  )
}
