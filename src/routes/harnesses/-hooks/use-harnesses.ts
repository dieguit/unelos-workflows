import { useState } from "react"
import { toast } from "sonner"
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
  const [harnesses, setHarnesses] = useState(initial)
  const [pending, setPending] = useState<Record<string, HarnessAction>>({})
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [scanning, setScanning] = useState(false)

  const run = async (h: Harness, action: HarnessAction) => {
    setPending((p) => ({ ...p, [h.id]: action }))
    setErrors((r) => without(r, h.id))
    try {
      setHarnesses(
        await (action === "uninstall" ? uninstallHarness : installHarness)(h.id)
      )
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

  return { harnesses, pending, errors, scanning, run, rescan }
}
