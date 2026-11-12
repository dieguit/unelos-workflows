import { Loader2Icon } from "lucide-react"
import { cn } from "cn"

export function BusyContent({ busy, children }: { busy: boolean; children: React.ReactNode }) {
  return (
    <span className="relative inline-flex items-center gap-[inherit]">
      <span className={cn("inline-flex items-center gap-[inherit]", busy && "invisible")}>{children}</span>
      {busy && <Loader2Icon aria-label="Saving" className="absolute inset-0 m-auto animate-spin" />}
    </span>
  )
}
