import { FlagIcon, SplitIcon, SquareTerminalIcon, UserCheckIcon, WorkflowIcon } from "lucide-react"
import type { WorkflowStep } from "@/lib/projects"

export const stepIcons = {
  task: SquareTerminalIcon,
  parallel: SplitIcon,
  workflow: WorkflowIcon,
  "human-gate": UserCheckIcon,
  end: FlagIcon,
} satisfies Record<WorkflowStep["kind"], unknown>

export const stepKindLabels = {
  task: "Task",
  parallel: "Parallel",
  workflow: "Workflow",
  "human-gate": "Human gate",
  end: "End",
} satisfies Record<WorkflowStep["kind"], string>

export const stepIconColors = {
  task: "text-primary",
  parallel: "text-pending",
  workflow: "text-primary",
  "human-gate": "text-chart-2",
  end: "text-muted-foreground",
} satisfies Record<WorkflowStep["kind"], string>
