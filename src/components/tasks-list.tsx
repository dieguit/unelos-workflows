import { ListTodoIcon, PlusIcon, SearchXIcon } from "lucide-react"
import type { ReactNode } from "react"
import { Button } from "@/components/ui/button"
import { EmptyState } from "@/components/empty-state"
import {
  ListBody,
  ListCell,
  ListHead,
  ListHeader,
  ListPrimary,
  ListRow,
  ListTable,
  Truncated,
} from "@/components/list-table"
import { UsedByCell, UsedByHead } from "@/components/used-by"
import type { UsedBy } from "@/lib/references"
import type { TaskSummary } from "@/lib/projects"

export function filterTasks(tasks: TaskSummary[], query: string) {
  const needle = query.trim().toLowerCase()
  return tasks.filter((task) => task.name.toLowerCase().includes(needle))
}

function List({ items }: { items: string[] }) {
  return items.length ? <Truncated text={items.join(", ")} /> : "—"
}

type Selection = {
  usedBy: UsedBy
  selected?: string
  onSelect?: (name: string) => void
  actions?: (name: string) => ReactNode
}

function TasksTable({
  tasks,
  usedBy,
  selected,
  onSelect,
  actions,
}: { tasks: TaskSummary[] } & Selection) {
  return (
    <ListTable>
      <ListHeader>
        <ListHead className="w-[26%]">Name</ListHead>
        <ListHead>Description</ListHead>
        <ListHead className="w-[18%]">Skills</ListHead>
        <ListHead className="w-[18%]">Outcomes</ListHead>
        <UsedByHead className="w-28" project={usedBy.project} />
        {actions && (
          <ListHead className="w-14">
            <span className="sr-only">Actions</span>
          </ListHead>
        )}
      </ListHeader>
      <ListBody>
        {tasks.map((t) => (
          <ListRow
            key={t.name}
            aria-selected={t.name === selected}
            className="cursor-pointer aria-selected:bg-primary-muted"
            onClick={() => onSelect?.(t.name)}
            onKeyDown={(e) => {
              if (e.target !== e.currentTarget) return
              if (e.key !== "Enter" && e.key !== " ") return
              e.preventDefault()
              onSelect?.(t.name)
            }}
          >
            <ListCell>
              <ListPrimary title={t.name} problem={t.problem} />
            </ListCell>
            <ListCell className="text-foreground-dim">
              {t.description ? <Truncated text={t.description} /> : "—"}
            </ListCell>
            <ListCell className="text-foreground-dim">
              <List items={t.skills} />
            </ListCell>
            <ListCell className="font-mono text-xs text-foreground-dim">
              <List items={t.outcomes} />
            </ListCell>
            <UsedByCell refs={usedBy.of(t.name)} />
            {actions && <ListCell>{actions(t.name)}</ListCell>}
          </ListRow>
        ))}
      </ListBody>
    </ListTable>
  )
}

export function TasksList({
  tasks,
  query,
  onClear,
  onCreate,
  ...selection
}: {
  tasks: TaskSummary[]
  query: string
  onClear: () => void
  onCreate?: () => void
} & Selection) {
  const shown = filterTasks(tasks, query)
  return tasks.length === 0 ? (
    <EmptyState
      dashed
      icon={ListTodoIcon}
      title="No tasks yet"
      description="Reusable tasks appear here. Workflow steps can use them instead of writing instructions inline."
      action={
        onCreate ? (
          <Button variant="outline" size="sm" onClick={onCreate}>
            <PlusIcon aria-hidden="true" />
            New Task
          </Button>
        ) : undefined
      }
    />
  ) : shown.length === 0 ? (
    <EmptyState
      icon={SearchXIcon}
      title={`No tasks match “${query.trim()}”`}
      action={
        <Button variant="outline" size="sm" onClick={onClear}>
          Clear search
        </Button>
      }
    />
  ) : (
    <TasksTable tasks={shown} {...selection} />
  )
}
