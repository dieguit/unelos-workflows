import type { ReactNode } from "react"
import { createFileRoute } from "@tanstack/react-router"
import { z } from "zod"
import { PageHeader } from "@/components/page-header"
import { SkillsError, SkillsPage } from "@/components/skills-page"
import {
  listSkills,
  createSkill,
  duplicateSkill,
  removeSkill,
  getSkill,
  makeSkillGlobal,
  requireProject,
  saveSkill,
} from "@/lib/projects"

export const Route = createFileRoute("/skills/")({
  validateSearch: z.object({ skill: z.string().optional().catch(undefined) }),
  beforeLoad: requireProject,
  loader: async ({ context }) => ({
    root: context.project.path,
    skills: await listSkills(context.project.path),
  }),
  component: SkillsRoute,
  pendingComponent: () => <p role="status">Loading skills…</p>,
  errorComponent: ({ error }) => <SkillsError error={error} header={Header} />,
})

function Header({ action }: { action?: ReactNode }) {
  return (
    <PageHeader
      title="Skills"
      description="Skill definitions for this project."
      action={action}
    />
  )
}

function SkillsRoute() {
  const { root, skills } = Route.useLoaderData()
  const { project } = Route.useRouteContext()
  const { skill } = Route.useSearch()
  const navigate = Route.useNavigate()
  if (root !== project.path)
    return (
      <div className="flex flex-col gap-6">
        <Header />
        <p role="status">Loading skills…</p>
      </div>
    )
  return (
    <SkillsPage
      key={project.path}
      skills={skills}
      id={skill}
      scope={root}
      header={Header}
      navigate={(o) => navigate(o)}
      api={{
        get: (id) => getSkill(root, id),
        save: (id, value) => saveSkill(root, id, value),
        duplicate: (id, newId) => duplicateSkill(root, id, newId),
        remove: (id) => removeSkill(root, id),
        makeGlobal: (id) => makeSkillGlobal(root, id),
        create: (name, description, body) =>
          createSkill(root, name, description, body),
      }}
    />
  )
}
