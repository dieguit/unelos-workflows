import { useEffect, useState } from "react"

export function useDelayedFlag(flag: boolean, ms = 400): boolean {
  const [elapsed, setElapsed] = useState(false)

  // Fast local saves should block interaction without flashing busy visuals.
  useEffect(() => {
    if (!flag) return
    const timer = setTimeout(() => setElapsed(true), ms)
    return () => {
      clearTimeout(timer)
      setElapsed(false)
    }
  }, [flag, ms])

  return flag && elapsed
}
