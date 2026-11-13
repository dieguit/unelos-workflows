import { useEffect, useRef, useState } from "react"
import { trackedInvoke } from "@/lib/tauri"
import type { ImportCommit, ImportPlan, ImportRenames } from "../-lib/import-plan"

export type ImportStep = "source" | "review" | "names"
type Imported = Extract<ImportCommit, { status: "imported" }>

const RENAME_DEBOUNCE_MS = 150

export function useWorkflowImport(root: string, onImported: (result: Imported) => void) {
  const [step, setStep] = useState<ImportStep>("source")
  const [source, setSourceText] = useState("")
  const [preparing, setPreparing] = useState(false)
  const [planning, setPlanning] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [importing, setImporting] = useState(false)
  const [failure, setFailure] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [plan, setPlan] = useState<ImportPlan>()
  const [renames, setRenames] = useState<ImportRenames>({})
  const [trust, setTrust] = useState(false)
  const id = useRef(crypto.randomUUID())
  const prepared = useRef<string | undefined>(undefined)
  const run = useRef(0)
  const timer = useRef<number | undefined>(undefined)

  const cancelRemote = () => void trackedInvoke("import_cancel", { id: id.current }).catch(() => {})
  useEffect(() => () => window.clearTimeout(timer.current), [])

  function setSource(value: string) {
    setSourceText(value)
    if (plan && value.trim() !== prepared.current) {
      cancelRemote()
      setPlan(undefined)
      prepared.current = undefined
    }
    setError(null)
    setTrust(false)
  }

  async function prepare() {
    const text = source.trim()
    if (!text || preparing) return
    if (plan && prepared.current === text) return setStep("review")
    const mine = ++run.current
    id.current = crypto.randomUUID()
    setPreparing(true)
    setError(null)
    try {
      const next = await trackedInvoke<ImportPlan>("import_prepare", { root, id: id.current, source: text })
      if (mine !== run.current) return
      prepared.current = text
      setPlan(next)
      setRenames({})
      setTrust(false)
      setNotice(null)
      setStep("review")
    } catch (e) {
      if (mine === run.current) {
        setPlan(undefined)
        setError(String(e))
      }
    } finally {
      if (mine === run.current) setPreparing(false)
    }
  }

  function rename(key: string, value: string) {
    const next = { ...renames, [key]: value }
    setRenames(next)
    setFailure(null)
    setPlanning(true)
    window.clearTimeout(timer.current)
    const mine = ++run.current
    timer.current = window.setTimeout(async () => {
      try {
        const updated = await trackedInvoke<ImportPlan>("import_plan", { root, id: id.current, renames: next })
        if (mine === run.current) setPlan(updated)
      } catch (e) {
        if (mine === run.current) setNotice(String(e))
      } finally {
        if (mine === run.current) setPlanning(false)
      }
    }, RENAME_DEBOUNCE_MS)
  }

  async function commit() {
    if (!plan) return
    const mine = ++run.current
    const sent = Object.fromEntries(plan.conflicts.filter((c) => c.key in renames).map((c) => [c.key, renames[c.key]]))
    setImporting(true)
    setFailure(null)
    try {
      const result = await trackedInvoke<ImportCommit>("import_commit", { root, id: id.current, renames: sent })
      if (mine !== run.current) return
      if (result.status === "imported") return onImported(result)
      setPlan(result.plan)
      setNotice("The project changed while importing. Check the names below.")
      setStep("names")
    } catch (e) {
      if (mine === run.current) setFailure(String(e))
    } finally {
      if (mine === run.current) setImporting(false)
    }
  }

  function back() {
    setFailure(null)
    setStep(step === "names" ? "review" : "source")
  }

  function cancel() {
    run.current++
    window.clearTimeout(timer.current)
    cancelRemote()
  }

  return {
    step, setStep, importing, failure, back, source, setSource, preparing, planning, error, notice, plan, renames, trust, setTrust,
    prepare, rename, commit, cancel,
  }
}
