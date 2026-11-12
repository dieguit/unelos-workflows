import { useCallback, useEffect, useRef, useState } from "react"

/** Loads on mount and every `ms`; keeps the last good data when a refresh fails. */
export function usePolled<T>(load: () => Promise<T>, key: string, ms = 2000) {
  const [state, setState] = useState<{ key: string; data?: T; error?: string }>({ key })
  const loadRef = useRef(load)
  useEffect(() => {
    loadRef.current = load
  })
  const refresh = useCallback(() => {
    const requested = key
    return loadRef.current().then(
      (data) => setState({ key: requested, data }),
      (err) => setState((s) => ({ key: requested, data: s.key === requested ? s.data : undefined, error: String(err) }))
    )
  }, [key])
  useEffect(() => {
    void refresh()
    const timer = setInterval(() => void refresh(), ms)
    return () => clearInterval(timer)
  }, [refresh, ms])
  // Data from another key (e.g. another project) is never shown.
  const fresh = state.key === key
  return { data: fresh ? state.data : undefined, error: fresh ? state.error : undefined, refresh }
}
