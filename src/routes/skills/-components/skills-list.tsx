import { BookOpenIcon, SearchXIcon } from "lucide-react"
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
import type { SkillSummary } from "@/lib/projects"

export function filterSkills(skills: SkillSummary[], query: string) {
  const needle = query.trim().toLowerCase()
  return skills.filter((skill) =>
    [skill.name, skill.description ?? ""].some((text) =>
      text.toLowerCase().includes(needle)
    )
  )
}

type Selection = { selected?: string; onSelect?: (name: string) => void }

function SkillsTable({
  skills,
  selected,
  onSelect,
}: { skills: SkillSummary[] } & Selection) {
  return (
    <ListTable>
      <ListHeader>
        <ListHead className="w-65">Name</ListHead>
        <ListHead>Description</ListHead>
      </ListHeader>
      <ListBody>
        {skills.map((s) => (
          <ListRow
            key={s.name}
            aria-selected={s.name === selected}
            className="cursor-pointer aria-selected:bg-primary-muted"
            onClick={() => onSelect?.(s.name)}
            onKeyDown={(e) => {
              if (e.target !== e.currentTarget) return
              if (e.key !== "Enter" && e.key !== " ") return
              e.preventDefault()
              onSelect?.(s.name)
            }}
          >
            <ListCell>
              <ListPrimary title={s.name} problem={s.problem} />
            </ListCell>
            <ListCell className="text-foreground-dim">
              {s.description ? <Truncated text={s.description} /> : "—"}
            </ListCell>
          </ListRow>
        ))}
      </ListBody>
    </ListTable>
  )
}

export function SkillsList({
  skills,
  query,
  onClear,
  ...selection
}: {
  skills: SkillSummary[]
  query: string
  onClear: () => void
} & Selection) {
  const shown = filterSkills(skills, query)
  return skills.length === 0 ? (
    <EmptyState
      dashed
      icon={BookOpenIcon}
      title="No skills yet"
      description="Skills defined in this project appear here."
    />
  ) : shown.length === 0 ? (
    <EmptyState
      icon={SearchXIcon}
      title={`No skills match “${query.trim()}”`}
      action={
        <Button variant="outline" size="sm" onClick={onClear}>
          Clear search
        </Button>
      }
    />
  ) : (
    <SkillsTable skills={shown} {...selection} />
  )
}
