import {
  BookOpenIcon,
  BotIcon,
  CableIcon,
  ArrowLeftRightIcon,
  HistoryIcon,
  ListTodoIcon,
  MoonIcon,
  SettingsIcon,
  SunIcon,
  WorkflowIcon,
  type LucideIcon,
} from "lucide-react"
import { Link, useRouteContext } from "@tanstack/react-router"
import { useTheme } from "@/components/theme-provider"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"
import { ProjectSelector } from "./project-selector"

export const sections = [
  { id: "settings", label: "Project settings", icon: SettingsIcon },
  { id: "workflows", label: "Workflows", icon: WorkflowIcon },
  { id: "agents", label: "Agents", icon: BotIcon },
  { id: "skills", label: "Skills", icon: BookOpenIcon },
  { id: "tasks", label: "Tasks", icon: ListTodoIcon },
  { id: "runs", label: "Runs", icon: HistoryIcon },
] as const

const rowClass =
  "flex h-8 w-full items-center gap-2.5 rounded-md px-2 text-base text-foreground-dim outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring"
const linkClass = cn(
  rowClass,
  "group data-[status=active]:bg-primary-muted data-[status=active]:font-medium data-[status=active]:text-foreground"
)
const iconClass =
  "size-4 shrink-0 text-muted-foreground group-data-[status=active]:text-primary"
const disabledClass = cn(
  rowClass,
  "cursor-default text-foreground-disabled hover:bg-transparent"
)

function DisabledRow({
  id,
  label,
  icon: Icon,
  reason,
}: {
  id: string
  label: string
  icon: LucideIcon
  reason: string
}) {
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <span
            role="link"
            aria-disabled="true"
            data-section={id}
            className={disabledClass}
          />
        }
      >
        <Icon aria-hidden="true" className="size-4 shrink-0" />
        {label}
      </TooltipTrigger>
      <TooltipContent side="right">{reason}</TooltipContent>
    </Tooltip>
  )
}

export function AppSidebar() {
  const { toggleTheme } = useTheme()
  const { project } = useRouteContext({ from: "__root__" })
  const renderSection = ({
    id,
    label,
    icon: Icon,
  }: (typeof sections)[number]) =>
    !project ? (
      <DisabledRow
        key={id}
        id={id}
        label={label}
        icon={Icon}
        reason="Open a project first"
      />
    ) : (
      <Link key={id} to={`/${id}`} data-section={id} className={linkClass}>
        <Icon aria-hidden="true" className={iconClass} />
        {label}
      </Link>
    )
  return (
    <aside
      aria-label="Project sidebar"
      className="flex h-full w-[240px] shrink-0 flex-col gap-4 border-r bg-sidebar px-2 py-3"
    >
      <div className="flex items-center gap-2 px-2 py-0.5">
        <span
          aria-hidden="true"
          className="flex size-5 items-center justify-center rounded-sm bg-primary text-xs font-bold text-primary-foreground"
        >
          U
        </span>
        <span className="text-base font-semibold">Unelos</span>
      </div>
      <ProjectSelector />
      <nav aria-label="Project sections" className="flex flex-col gap-0.5">
        {sections.map(renderSection)}
      </nav>
      <nav aria-labelledby="sidebar-global" className="flex flex-col gap-0.5">
        <h2
          id="sidebar-global"
          className="border-t px-2 pt-3 pb-1 text-xs font-medium tracking-[0.6px] text-muted-foreground uppercase"
        >
          Global
        </h2>
        <Link to="/harnesses" data-section="harnesses" className={linkClass}>
          <CableIcon aria-hidden="true" className={iconClass} />
          Harnesses
        </Link>
        <Link
          to="/model-mappings"
          data-section="model-mappings"
          className={linkClass}
        >
          <ArrowLeftRightIcon aria-hidden="true" className={iconClass} />
          Model mappings
        </Link>
        <Link to="/global-tasks" data-section="global-tasks" className={linkClass}>
          <ListTodoIcon aria-hidden="true" className={iconClass} />
          Global tasks
        </Link>
      </nav>
      <footer className="mt-auto flex flex-col gap-0.5 border-t pt-2">
        <button
          type="button"
          data-theme-toggle
          onClick={toggleTheme}
          className={rowClass}
        >
          <MoonIcon
            aria-hidden="true"
            className="size-4 shrink-0 text-muted-foreground dark:hidden"
          />
          <SunIcon
            aria-hidden="true"
            className="hidden size-4 shrink-0 text-muted-foreground dark:block"
          />
          <span className="dark:hidden">Dark mode</span>
          <span className="hidden dark:inline">Light mode</span>
        </button>
      </footer>
    </aside>
  )
}
