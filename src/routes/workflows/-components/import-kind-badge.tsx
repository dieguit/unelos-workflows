import { BotIcon, ListTodoIcon, PlugIcon, SparklesIcon, WorkflowIcon, type LucideIcon } from "lucide-react"
import { kindLabel, type ImportKind } from "../-lib/import-plan"

export const kindIcon: Record<ImportKind, LucideIcon> = {
  workflow: WorkflowIcon,
  agent: BotIcon,
  task: ListTodoIcon,
  skill: SparklesIcon,
  mcp: PlugIcon,
}

export function ImportKindBadge({ kind }: { kind: ImportKind }) {
  const Icon = kindIcon[kind]
  return (
    <span className="inline-flex h-6 w-[4.5rem] items-center gap-1.5 rounded-md bg-muted px-2 text-xs text-foreground-dim">
      <Icon aria-hidden className="size-3.5 shrink-0" />
      {kindLabel[kind]}
    </span>
  )
}
