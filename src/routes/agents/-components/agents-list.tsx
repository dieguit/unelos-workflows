import { BotIcon, SearchXIcon } from "lucide-react"
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
import type { AgentSummary } from "@/lib/projects"

export function filterAgents(agents: AgentSummary[], query: string) {
  const needle = query.trim().toLowerCase()
  return agents.filter((agent) => agent.name.toLowerCase().includes(needle))
}

function List({ items }: { items: string[] }) {
  return items.length ? <Truncated text={items.join(", ")} /> : "—"
}

function AgentsTable({ agents }: { agents: AgentSummary[] }) {
  return (
    <ListTable>
      <ListHeader>
        <ListHead className="w-65">Name</ListHead>
        <ListHead>Model</ListHead>
        <ListHead className="w-45">Skills</ListHead>
        <ListHead className="w-45">MCPs</ListHead>
      </ListHeader>
      <ListBody>
        {agents.map((a) => (
          <ListRow key={a.name}>
            <ListCell>
              <ListPrimary title={a.name} problem={a.problem} />
            </ListCell>
            <ListCell className="text-foreground-dim">
              {a.model ? (
                <Truncated text={a.model} />
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
}: {
  agents: AgentSummary[]
  query: string
  onClear: () => void
}) {
  const shown = filterAgents(agents, query)
  return agents.length === 0 ? (
    <EmptyState
      dashed
      icon={BotIcon}
      title="No agents yet"
      description="Agents defined in this project appear here."
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
    <AgentsTable agents={shown} />
  )
}
