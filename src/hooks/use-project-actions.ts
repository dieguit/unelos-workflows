import { useRouter } from "@tanstack/react-router"
import { revealItemInDir } from "@tauri-apps/plugin-opener"
import { toast } from "sonner"
import { openProject, pickFolder, removeRecentProject } from "@/lib/projects"

export function useProjectActions() {
  const router = useRouter()
  const fail = (error: unknown) => void toast.error(String(error))

  const openPath = async (path: string) => {
    try {
      await openProject(path)
      await router.invalidate()
      await router.navigate({ to: "/workflows" })
    } catch (error) {
      fail(error)
    }
  }

  return {
    openPath,
    pickAndOpen: async () => {
      try {
        const path = await pickFolder()
        if (path) await openPath(path)
      } catch (error) {
        fail(error)
      }
    },
    removeRecent: async (path: string) => {
      try {
        await removeRecentProject(path)
        await router.invalidate()
      } catch (error) {
        fail(error)
      }
    },
    copyPath: (path: string) =>
      navigator.clipboard
        .writeText(path)
        .then(() => void toast.success("Path copied"), fail),
    reveal: (path: string) => revealItemInDir(path).catch(fail),
  }
}
