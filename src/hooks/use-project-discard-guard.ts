import { useEffect, useRef } from "react"
import { registerProjectChangeGuard } from "@/lib/project-change-guard"

export function useProjectDiscardGuard(busy: boolean, dirty: boolean, ask: () => Promise<boolean>) {
  const approved = useRef(false)
  useEffect(() => registerProjectChangeGuard(async () => {
    if (busy || (dirty && !(await ask()))) return null
    approved.current = true
    return () => { approved.current = false }
  }), [busy, dirty, ask])
  return approved
}
