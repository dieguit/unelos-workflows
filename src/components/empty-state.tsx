import type { ReactNode } from "react"
import type { LucideIcon } from "lucide-react"
import { cn } from "@/lib/utils"

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  dashed,
}: {
  icon: LucideIcon
  title: ReactNode
  description?: ReactNode
  action?: ReactNode
  dashed?: boolean
}) {
  return (
    <div
      className={cn(
        "flex flex-col items-center gap-3 py-16 text-center",
        dashed && "rounded-lg border border-dashed"
      )}
    >
      <Icon
        aria-hidden="true"
        className={cn(
          "size-6",
          dashed ? "text-primary" : "text-muted-foreground"
        )}
      />
      {dashed ? (
        <h2 className="text-lg font-semibold">{title}</h2>
      ) : (
        <p className="text-base">{title}</p>
      )}
      {description ? (
        <p className="text-sm text-foreground-dim">{description}</p>
      ) : null}
      {action}
    </div>
  )
}
