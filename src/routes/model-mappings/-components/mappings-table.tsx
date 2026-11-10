import {
  ArrowLeftRightIcon,
  CheckIcon,
  SearchXIcon,
  TriangleAlertIcon,
} from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { EmptyState } from "@/components/empty-state"
import {
  ListBody,
  ListCell,
  ListHead,
  ListHeader,
  ListRow,
  ListTable,
} from "@/components/list-table"
import type { ModelMappings } from "@/lib/model-mappings"
import type { AgentSummary } from "@/lib/projects"
import {
  HARNESSES,
  filterMappings,
  usedBy,
  usedByLabel,
} from "../-lib/model-mappings-list"

function HarnessChip({ name, model }: { name: string; model?: string }) {
  return model ? (
    <Badge variant="outline" title={model} className="rounded-md">
      <CheckIcon aria-hidden="true" className="text-success" />
      {name}
    </Badge>
  ) : (
    <Badge
      title="Not mapped"
      className="rounded-md border-warning/40 bg-warning-muted text-warning"
    >
      <TriangleAlertIcon aria-hidden="true" />
      {name}
      <span className="sr-only"> not mapped</span>
    </Badge>
  )
}

export function MappingsTable({
  mappings,
  agents,
  query,
  onClear,
  onCreate,
  selected,
  onSelect,
}: {
  mappings: ModelMappings
  agents: AgentSummary[] | null
  query: string
  onClear: () => void
  onCreate: () => void
  selected?: string
  onSelect: (alias: string) => void
}) {
  const shown = filterMappings(mappings, query)
  if (Object.keys(mappings).length === 0)
    return (
      <EmptyState
        dashed
        icon={ArrowLeftRightIcon}
        title="No aliases yet"
        description="Create an alias, then map it to a model for each harness."
        action={
          <Button size="sm" onClick={onCreate}>
            New alias
          </Button>
        }
      />
    )
  if (shown.length === 0)
    return (
      <EmptyState
        icon={SearchXIcon}
        title={`No aliases match “${query.trim()}”`}
        action={
          <Button variant="outline" size="sm" onClick={onClear}>
            Clear search
          </Button>
        }
      />
    )
  return (
    <ListTable>
      <ListHeader>
        <ListHead className="w-56">Alias</ListHead>
        <ListHead>Harnesses</ListHead>
        <ListHead className="w-28">Used by</ListHead>
      </ListHeader>
      <ListBody>
        {shown.map((alias) => (
          <ListRow
            key={alias}
            aria-selected={alias === selected}
            className="cursor-pointer aria-selected:bg-primary-muted"
            onClick={() => onSelect(alias)}
            onKeyDown={(e) => {
              if (e.target !== e.currentTarget) return
              if (e.key !== "Enter" && e.key !== " ") return
              e.preventDefault()
              onSelect(alias)
            }}
          >
            <ListCell>
              <span className="block truncate text-base font-medium text-primary">
                {alias}
              </span>
            </ListCell>
            <ListCell>
              <span className="flex flex-wrap gap-1">
                {HARNESSES.map((h) => (
                  <HarnessChip
                    key={h.id}
                    name={h.name}
                    model={mappings[alias][h.id]}
                  />
                ))}
              </span>
            </ListCell>
            <ListCell className="text-foreground-dim">
              {usedByLabel(usedBy(agents, alias))}
            </ListCell>
          </ListRow>
        ))}
      </ListBody>
    </ListTable>
  )
}
