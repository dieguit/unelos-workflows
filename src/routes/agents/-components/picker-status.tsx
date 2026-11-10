import { Loader2Icon } from "lucide-react"
import { Button } from "@/components/ui/button"

/** Loading / error body for a lazily loaded picker list. */
export function PickerStatus({
  what,
  error,
  onRetry,
}: {
  what: string
  error?: string
  onRetry: () => void
}) {
  return error ? (
    <div role="alert" className="flex flex-col items-start gap-2 p-3">
      <p className="text-sm [overflow-wrap:anywhere] text-destructive">
        Couldn’t load {what}: {error}
      </p>
      <Button variant="outline" size="sm" onClick={onRetry}>
        Retry
      </Button>
    </div>
  ) : (
    <p
      role="status"
      className="flex items-center justify-center gap-2 py-6 text-sm text-muted-foreground"
    >
      <Loader2Icon aria-hidden="true" className="size-4 animate-spin" />
      Loading {what}…
    </p>
  )
}
