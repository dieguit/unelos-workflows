import { useEffect, useState } from "react"
import type { SkillDocument } from "@/lib/projects"

type Loaded = { id: string; doc?: SkillDocument; error?: string }

/** Loads one skill; `setDoc` replaces it after a save (keyed by the saved name, so a rename needs no refetch flash). `scope` reloads when the project changes. */
export function useSkill(
  scope: string,
  get: (id: string) => Promise<SkillDocument>,
  id: string | undefined
) {
  const [state, setState] = useState<Loaded>()
  useEffect(() => {
    if (!id) return
    let live = true
    get(id).then(
      (doc) => live && setState({ id, doc }),
      (error) => live && setState({ id, error: String(error) })
    )
    return () => {
      live = false
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scope, id])
  const current = state?.id === id ? state : undefined
  return {
    doc: current?.doc,
    error: current?.error,
    setDoc: (doc: SkillDocument) => setState({ id: doc.name, doc }),
  }
}
