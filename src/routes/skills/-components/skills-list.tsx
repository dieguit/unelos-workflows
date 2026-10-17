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

function SkillsTable({ skills }: { skills: SkillSummary[] }) {
  return (
    <ListTable>
      <ListHeader>
        <ListHead className="w-65">Name</ListHead>
        <ListHead>Description</ListHead>
      </ListHeader>
      <ListBody>
        {skills.map((s) => (
          <ListRow key={s.name}>
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
}: {
  skills: SkillSummary[]
  query: string
  onClear: () => void
}) {
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
    <SkillsTable skills={shown} />
  )
}
