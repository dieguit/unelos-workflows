import type { ReactNode } from "react"
import { createFileRoute } from "@tanstack/react-router"
import { z } from "zod"
import { PageHeader } from "@/components/page-header"
import { loadReferences, usedByFor } from "@/lib/references"
import { SkillsError, SkillsPage } from "@/components/skills-page"
import {
  createGlobalSkill,
  duplicateGlobalSkill,
  getGlobalSkill,
  listGlobalSkills,
  removeGlobalSkill,
  saveGlobalSkill,
} from "@/lib/projects"

export const Route = createFileRoute("/global-skills/")({
  validateSearch: z.object({ skill: z.string().optional().catch(undefined) }),
  loader: async ({ context }) => ({
    skills: await listGlobalSkills(),
    references: await loadReferences(context.project?.path),
  }),
  component: GlobalSkillsRoute,
  pendingComponent: () => <p role="status">Loading global skills…</p>,
  errorComponent: ({ error }) => <SkillsError error={error} header={Header} />,
})

function Header({ action }: { action?: ReactNode }) {
  return (
    <PageHeader
      title="Global Skills"
      description="Skills available to every project on this machine. Stored in ~/.config/unelos."
      action={action}
    />
  )
}

function GlobalSkillsRoute() {
  const { skills, references } = Route.useLoaderData()
  const { skill } = Route.useSearch()
  const { project } = Route.useRouteContext()
  const navigate = Route.useNavigate()
  const root = project?.path ?? null
  return (
    <SkillsPage
      skills={skills}
      usedBy={usedByFor(references?.skills, { global: true, project: project?.name })}
      id={skill}
      scope="global"
      header={Header}
      navigate={(o) => navigate(o)}
      api={{
        get: getGlobalSkill,
        save: (id, value) => saveGlobalSkill(root, id, value),
        duplicate: duplicateGlobalSkill,
        remove: (id) => removeGlobalSkill(root, id),
        create: createGlobalSkill,
      }}
    />
  )
}
