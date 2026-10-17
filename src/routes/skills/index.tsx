import { useRef, useState } from "react"
import { createFileRoute, useRouter } from "@tanstack/react-router"
import { Button } from "@/components/ui/button"
import { ListSearch } from "@/components/list-search"
import { PageHeader } from "@/components/page-header"
import { listSkills, requireProject } from "@/lib/projects"
import { SkillsList, filterSkills } from "./-components/skills-list"

export const Route = createFileRoute("/skills/")({
  beforeLoad: requireProject,
  loader: async ({ context }) => ({
    root: context.project.path,
    skills: await listSkills(context.project.path),
  }),
  component: SkillsRoute,
  pendingComponent: () => <p role="status">Loading skills…</p>,
  errorComponent: SkillsError,
})

function Header() {
  return (
    <PageHeader title="Skills" description="Skill definitions for this project." />
  )
}

function SkillsRoute() {
  const { project } = Route.useRouteContext()
  return <SkillsHome key={project.path} />
}

function SkillsHome() {
  const { root, skills } = Route.useLoaderData()
  const { project } = Route.useRouteContext()
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)

  if (root !== project.path)
    return (
      <div className="flex flex-col gap-6">
        <Header />
        <p role="status">Loading skills…</p>
      </div>
    )

  return (
    <div className="flex flex-col gap-4">
      <Header />
      {skills.length > 0 ? (
        <ListSearch
          noun="skill"
          value={query}
          onChange={setQuery}
          shown={filterSkills(skills, query).length}
          total={skills.length}
          inputRef={search}
        />
      ) : null}
      <SkillsList
        skills={skills}
        query={query}
        onClear={() => {
          setQuery("")
          search.current?.focus()
        }}
      />
    </div>
  )
}

function SkillsError({ error }: { error: unknown }) {
  const router = useRouter()
  return (
    <div className="flex flex-col gap-6">
      <Header />
      <div className="flex flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
        <p className="text-sm [overflow-wrap:anywhere]">
          Couldn’t load skills:{" "}
          {error instanceof Error ? error.message : String(error)}
        </p>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void router.invalidate()}
        >
          Retry
        </Button>
      </div>
    </div>
  )
}
