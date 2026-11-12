import { createFileRoute } from "@tanstack/react-router"
import { PageHeader } from "@/components/page-header"
import { getProjectSettings, requireProject } from "@/lib/projects"
import { HarnessBuildSettings } from "./-components/harness-build-settings"

export const Route = createFileRoute("/settings/")({
  beforeLoad: requireProject,
  loader: async ({ context }) => ({
    root: context.project.path,
    settings: await getProjectSettings(context.project.path),
  }),
  component: SettingsHome,
  pendingComponent: () => <p role="status">Loading settings…</p>,
})

function SettingsHome() {
  const { root, settings } = Route.useLoaderData()
  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title="Project settings"
        description="Options for this project."
      />
      <HarnessBuildSettings key={root} root={root} initial={settings} />
    </div>
  )
}
