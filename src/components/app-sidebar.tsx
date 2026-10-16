import {
  BookOpenIcon,
  BotIcon,
  ChevronsUpDownIcon,
  FolderGit2Icon,
  HistoryIcon,
  MoonIcon,
  SettingsIcon,
  SunIcon,
  WorkflowIcon,
} from "lucide-react"
import { useTheme } from "@/components/theme-provider"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"

export const sections = [
  { id: "workflows", label: "Workflows", icon: WorkflowIcon },
  { id: "agents", label: "Agents", icon: BotIcon },
  { id: "skills", label: "Skills", icon: BookOpenIcon },
  { id: "runs", label: "Runs", icon: HistoryIcon },
  { id: "settings", label: "Project settings", icon: SettingsIcon },
] as const

export type SectionId = (typeof sections)[number]["id"]

const rowClass =
  "flex h-8 w-full items-center gap-2.5 rounded-md px-2 text-base text-foreground-dim outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring"

export function AppSidebar({
  activeSection,
  onSelect,
}: {
  activeSection: SectionId
  onSelect: (id: SectionId) => void
}) {
  const { toggleTheme } = useTheme()
  const renderSection = ({
    id,
    label,
    icon: Icon,
  }: (typeof sections)[number]) => (
    <button
      key={id}
      type="button"
      data-section={id}
      aria-current={activeSection === id ? "page" : undefined}
      onClick={() => onSelect(id)}
      className={cn(
        rowClass,
        activeSection === id && "bg-primary-muted font-medium text-foreground"
      )}
    >
      <Icon
        aria-hidden="true"
        className={cn(
          "size-4 shrink-0 text-muted-foreground",
          activeSection === id && "text-primary"
        )}
      />
      {label}
    </button>
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
      <Tooltip>
        <TooltipTrigger
          render={
            <button
              type="button"
              aria-disabled="true"
              aria-label="Sample project unelos-app at /home/dieguit/projects/unelos/unelos-app; project selection is not wired yet"
              className="flex w-full items-center gap-2.5 rounded-md border bg-card p-2 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
          }
        >
          <span className="flex size-7 shrink-0 items-center justify-center rounded-sm bg-secondary">
            <FolderGit2Icon
              aria-hidden="true"
              className="size-4 text-primary"
            />
          </span>
          <span className="min-w-0 flex-1">
            <span className="block truncate text-base font-semibold">
              unelos-app
            </span>
            <span className="block truncate font-mono text-xs text-muted-foreground">
              ~/…/unelos-app
            </span>
          </span>
          <ChevronsUpDownIcon
            aria-hidden="true"
            className="size-3.5 shrink-0 text-muted-foreground"
          />
        </TooltipTrigger>
        <TooltipContent side="right">
          Sample project: /home/dieguit/projects/unelos/unelos-app. Project
          selection is not wired yet.
        </TooltipContent>
      </Tooltip>
      <nav aria-label="Project sections" className="flex flex-col gap-0.5">
        {sections.slice(0, 4).map(renderSection)}
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
        {renderSection(sections[4])}
      </footer>
    </aside>
  )
}
