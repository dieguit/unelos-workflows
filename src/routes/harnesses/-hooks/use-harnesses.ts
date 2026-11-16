import { useState } from "react"
import { toast } from "sonner"
import { useRouteContext } from "@tanstack/react-router"
import { autoBuildHarness, listAgents } from "@/lib/projects"
import { listModelMappings } from "@/lib/model-mappings"
import { missingAliases, type MissingAlias } from "../-lib/missing-mappings"
import {
  installHarness,
  listHarnesses,
  uninstallHarness,
  type Harness,
} from "@/lib/harnesses"

export type HarnessAction = "install" | "update" | "reinstall" | "uninstall"

const without = <T>(record: Record<string, T>, key: string) => {
  const next = { ...record }
  delete next[key]
  return next
}

export function useHarnesses(initial: Harness[]) {
  const { project } = useRouteContext({ from: "__root__" })
  const [harnesses, setHarnesses] = useState(initial)
  const [pending, setPending] = useState<Record<string, HarnessAction>>({})
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [scanning, setScanning] = useState(false)
  const [mapping, setMapping] = useState<{ harness: Harness; action: HarnessAction; rows: MissingAlias[] }>()

  const build = async (action: HarnessAction) => {
    if (!project) return
    try {
      await autoBuildHarness(project.path)
    } catch (e) {
      toast.error(`Harness ${action} succeeded, but harness config build failed: ${String(e)}`)
    }
  }

  const missingFor = async (h: Harness) => {
    try {
      const agents = project ? await listAgents(project.path).catch(() => null) : null
      return missingAliases(await listModelMappings(), h.id, agents)
    } catch {
      return []
    }
  }

  const run = async (h: Harness, action: HarnessAction) => {
    setPending((p) => ({ ...p, [h.id]: action }))
    setErrors((r) => without(r, h.id))
    try {
      setHarnesses(
        await (action === "uninstall" ? uninstallHarness : installHarness)(h.id)
      )
      const rows = action === "uninstall" ? [] : await missingFor(h)
      // Build waits for the dialog so the new mappings are included.
      if (rows.length) setMapping({ harness: h, action, rows })
      else await build(action)
    } catch (e) {
      setErrors((r) => ({
        ...r,
        [h.id]: `Couldn’t ${action} ${h.name}: ${String(e)}`,
      }))
    } finally {
      setPending((p) => without(p, h.id))
    }
  }

  const rescan = async () => {
    setScanning(true)
    try {
      setHarnesses(await listHarnesses())
      setErrors({})
    } catch (e) {
      toast.error(`Couldn’t rescan harnesses: ${String(e)}`)
    } finally {
      setScanning(false)
    }
  }

  const closeMapping = async () => {
    const m = mapping
    setMapping(undefined)
    if (m) await build(m.action)
  }

  return { harnesses, pending, errors, scanning, run, rescan, mapping, closeMapping }
}
