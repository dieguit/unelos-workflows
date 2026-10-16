import { useRouter } from "@tanstack/react-router"
import { revealItemInDir } from "@tauri-apps/plugin-opener"
import { toast } from "sonner"
import { openProject, pickProject, removeRecentProject } from "@/lib/projects"

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
        const state = await pickProject()
        if (state) {
          await router.invalidate()
          await router.navigate({ to: "/workflows" })
        }
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
    copyPath: async (path: string) => {
      try {
        await navigator.clipboard.writeText(path)
        toast.success("Path copied")
      } catch (error) {
        fail(error)
      }
    },
    reveal: (path: string) => revealItemInDir(path).catch(fail),
  }
}
