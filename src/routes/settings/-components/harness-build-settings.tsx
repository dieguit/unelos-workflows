import { useState } from "react"
import { Link } from "@tanstack/react-router"
import { Loader2Icon, HammerIcon } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import {
  buildHarnessConfig,
  saveProjectSettings,
  type ProjectSettings,
} from "@/lib/projects"

export function HarnessBuildSettings({
  root,
  initial,
}: {
  root: string
  initial: ProjectSettings
}) {
  const [auto, setAuto] = useState(initial.autoBuildHarness ?? true)
  const [building, setBuilding] = useState(false)
  const [noHarness, setNoHarness] = useState(false)

  const toggle = async (next: boolean) => {
    const previous = auto
    setAuto(next)
    try {
      await saveProjectSettings(root, { autoBuildHarness: next })
    } catch (e) {
      setAuto(previous)
      toast.error(`Couldn’t save settings: ${String(e)}`)
    }
  }

  const build = async () => {
    setBuilding(true)
    try {
      const r = await buildHarnessConfig(root)
      setNoHarness(r.harnesses.length === 0)
      const changed = r.written.length + r.removed.length
      toast.success(
        changed === 0
          ? "Harness config up to date"
          : `Harness config built: ${r.written.length} written, ${r.removed.length} removed`
      )
      if (r.skipped.length)
        toast.warning(`Skipped invalid workflows: ${r.skipped.join("; ")}`)
    } catch (e) {
      toast.error(`Harness config build failed: ${String(e)}`)
    } finally {
      setBuilding(false)
    }
  }

  return (
    <section className="flex max-w-2xl flex-col gap-4 rounded-lg border p-4">
      <div className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <label htmlFor="auto-build" className="text-base font-medium">
            Build harness config automatically when workflows change
          </label>
          <p className="text-sm text-foreground-dim">
            After saving a workflow, agent or MCP, unelos regenerates the
            harness’s project files (e.g. OpenCode agents in .opencode/agents
            and MCP entries in opencode.json). Some harnesses only pick up
            changes on restart.
          </p>
        </div>
        <Switch
          id="auto-build"
          checked={auto}
          onCheckedChange={(v) => void toggle(v)}
        />
      </div>
      <div className="flex items-center gap-3 border-t pt-4">
        <Button
          variant="outline"
          size="sm"
          disabled={building}
          onClick={() => void build()}
        >
          {building ? (
            <Loader2Icon aria-hidden="true" className="animate-spin" />
          ) : (
            <HammerIcon aria-hidden="true" />
          )}
          Build now
        </Button>
        {noHarness && (
          <p role="status" className="text-sm text-foreground-dim">
            No harness integration is installed.{" "}
            <Link to="/harnesses" className="underline">
              Install one in Harnesses
            </Link>
            .
          </p>
        )}
      </div>
    </section>
  )
}
