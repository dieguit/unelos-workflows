import { BotIcon, PlusIcon, SearchXIcon } from "lucide-react"
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
import type { AgentModel, AgentSummary } from "@/lib/projects"
import type { ReactNode } from "react"

export function filterAgents(agents: AgentSummary[], query: string) {
  const needle = query.trim().toLowerCase()
  return agents.filter((agent) => agent.name.toLowerCase().includes(needle))
}

function List({ items }: { items: string[] }) {
  return items.length ? <Truncated text={items.join(", ")} /> : "—"
}

/** Alias as its name; a harness-pinned model with the harness id muted. */
export function ModelName({ model }: { model: AgentModel }) {
  return typeof model === "string" ? (
    <Truncated text={model} />
  ) : (
    <span className="flex min-w-0 items-baseline gap-1.5">
      <Truncated text={model.model} className="min-w-0" />
      <span className="shrink-0 text-xs text-muted-foreground">
        {model.harness}
      </span>
    </span>
  )
}

type Selection = {
  selected?: string
  onSelect?: (name: string) => void
  actions?: (name: string) => ReactNode
}

function AgentsTable({
  agents,
  selected,
  onSelect,
  actions,
}: { agents: AgentSummary[] } & Selection) {
  return (
    <ListTable>
      <ListHeader>
        <ListHead className="w-[28%]">Name</ListHead>
        <ListHead>Model</ListHead>
        <ListHead className="w-[22%]">Skills</ListHead>
        <ListHead className="w-[18%]">MCPs</ListHead>
        {actions && (
          <ListHead className="w-14">
            <span className="sr-only">Actions</span>
          </ListHead>
        )}
      </ListHeader>
      <ListBody>
        {agents.map((a) => (
          <ListRow
            key={a.name}
            aria-selected={a.name === selected}
            className="cursor-pointer aria-selected:bg-primary-muted"
            onClick={() => onSelect?.(a.name)}
            onKeyDown={(e) => {
              if (e.target !== e.currentTarget) return
              if (e.key !== "Enter" && e.key !== " ") return
              e.preventDefault()
              onSelect?.(a.name)
            }}
          >
            <ListCell>
              <ListPrimary title={a.name} problem={a.problem} />
            </ListCell>
            <ListCell className="text-foreground-dim">
              {a.model ? (
                <ModelName model={a.model} />
              ) : a.problem ? (
                "—"
              ) : (
                <span className="text-muted-foreground">Not configured</span>
              )}
            </ListCell>
            <ListCell className="text-foreground-dim">
              <List items={a.skills} />
            </ListCell>
            <ListCell className="text-foreground-dim">
              <List items={a.mcps} />
            </ListCell>
            {actions && <ListCell>{actions(a.name)}</ListCell>}
          </ListRow>
        ))}
      </ListBody>
    </ListTable>
  )
}

export function AgentsList({
  agents,
  query,
  onClear,
  onCreate,
  ...selection
}: {
  agents: AgentSummary[]
  query: string
  onClear: () => void
  onCreate?: () => void
} & Selection) {
  const shown = filterAgents(agents, query)
  return agents.length === 0 ? (
    <EmptyState
      dashed
      icon={BotIcon}
      title="No agents yet"
      description="Agents defined in this project appear here."
      action={
        onCreate ? (
          <Button variant="outline" size="sm" onClick={onCreate}>
            <PlusIcon aria-hidden="true" />
            New Agent
          </Button>
        ) : undefined
      }
    />
  ) : shown.length === 0 ? (
    <EmptyState
      icon={SearchXIcon}
      title={`No agents match “${query.trim()}”`}
      action={
        <Button variant="outline" size="sm" onClick={onClear}>
          Clear search
        </Button>
      }
    />
  ) : (
    <AgentsTable agents={shown} {...selection} />
  )
}
