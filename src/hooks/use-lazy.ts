import { useCallback, useRef, useState } from "react"

/** Runs `fetch` on the first `load()` and caches the result for the page; retries after an error. */
export function useLazy<T>(fetch: () => Promise<T>) {
  const [state, setState] = useState<{ data?: T; error?: string }>({})
  const started = useRef(false)
  const load = useCallback(() => {
    if (started.current) return
    started.current = true
    setState({})
    fetch().then(
      (data) => setState({ data }),
      (e) => {
        started.current = false
        setState({ error: String(e) })
      }
    )
  }, [fetch])
  return { ...state, load }
}
