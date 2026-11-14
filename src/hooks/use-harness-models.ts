import { useCallback, useRef, useState } from "react"
import { listHarnessModels } from "@/lib/model-mappings"

export type HarnessModels = { models?: string[]; error?: string }

/** Lazily lists each harness's models once per page; `load` is a no-op while loading or once loaded, and retries after an error. */
export function useHarnessModels() {
  const [state, setState] = useState<Record<string, HarnessModels>>({})
  const started = useRef(new Set<string>())
  const load = useCallback((id: string) => {
    if (started.current.has(id)) return
    started.current.add(id)
    setState((s) => ({ ...s, [id]: {} }))
    listHarnessModels(id).then(
      (models) => setState((s) => ({ ...s, [id]: { models } })),
      (e) => {
        started.current.delete(id)
        setState((s) => ({ ...s, [id]: { error: String(e) } }))
      }
    )
  }, [])
  return { models: state, load }
}
