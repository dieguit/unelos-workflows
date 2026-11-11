import { useRef, useState } from "react"
import { PlusIcon } from "lucide-react"
import { cn } from "cn"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { stepIconColors, stepIcons, stepKindLabels } from "./step-icon"

export function FlowLine({ className = "h-3" }: { className?: string }) {
  return <span aria-hidden="true" className={`w-px bg-border-strong ${className}`} />
}

export function FlowArrow() {
  return (
    <svg aria-hidden="true" width="9" height="6" viewBox="0 0 9 6" className="-mt-px fill-border-strong">
      <path d="M0 0h9L4.5 6z" />
    </svg>
  )
}

type AddKind = "task" | "human-gate"

/** "Add step" pill; the type picker opens beside it so the transition stays visible. */
function AddStep({ onAdd }: { onAdd?: (kind: AddKind) => void }) {
  const [open, setOpen] = useState(false)
  const first = useRef<HTMLDivElement>(null)
  // Picking a type moves focus to the new step's Name; only Esc / outside click returns it to the pill.
  const picked = useRef(false)
  return (
    <DropdownMenu
      open={open}
      onOpenChange={(next) => {
        if (next) picked.current = false
        setOpen(next)
      }}
      onOpenChangeComplete={(next) => next && first.current?.focus()}
    >
      <DropdownMenuTrigger
        disabled={!onAdd}
        render={
          <Button
            variant="outline"
            size="xs"
            className="rounded-full border-border-strong bg-background text-muted-foreground aria-expanded:border-primary aria-expanded:bg-primary-muted aria-expanded:text-primary dark:border-border-strong dark:bg-background dark:aria-expanded:border-primary dark:aria-expanded:bg-primary-muted"
          />
        }
      >
        <PlusIcon /> Add step
      </DropdownMenuTrigger>
      <DropdownMenuContent
        side="right"
        align="center"
        sideOffset={8}
        finalFocus={() => !picked.current}
        className="w-auto min-w-44"
      >
        {(["task", "human-gate", "parallel"] as const).map((kind) => {
          const Icon = stepIcons[kind]
          const soon = kind === "parallel"
          return (
            <DropdownMenuItem
              key={kind}
              ref={kind === "task" ? first : undefined}
              disabled={soon}
              onClick={() => {
                if (soon) return
                picked.current = true
                onAdd?.(kind)
              }}
              className="h-8 gap-2.5"
            >
              <span className="flex size-5 items-center justify-center rounded-sm bg-muted">
                <Icon aria-hidden="true" className={cn("size-3.5", stepIconColors[kind])} />
              </span>
              {stepKindLabels[kind]}
              {soon && <span className="ml-auto pl-3 text-xs whitespace-nowrap text-muted-foreground">Coming soon</span>}
            </DropdownMenuItem>
          )
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

/** Edge to the immediately next step. Outcome labels only for declared outcomes. */
export function FlowConnector({
  outcomes,
  onAdd,
}: {
  outcomes: string[] | null
  onAdd?: (kind: AddKind) => void
}) {
  if (!outcomes) return <div aria-hidden="true" className="h-8" />
  return (
    <div className="flex flex-col items-center">
      <FlowLine />
      {outcomes.length > 0 && (
        <>
          <span className="rounded-sm border bg-background px-1.5 py-px font-mono text-xs text-muted-foreground">
            {outcomes.join(" · ")}
          </span>
          <FlowLine className="h-1.5" />
        </>
      )}
      <AddStep onAdd={onAdd} />
      <FlowLine className="h-2.5" />
      <FlowArrow />
    </div>
  )
}
