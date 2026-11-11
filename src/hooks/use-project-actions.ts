import { useRouter } from "@tanstack/react-router"
import { revealItemInDir } from "@tauri-apps/plugin-opener"
import { toast } from "sonner"
import { openProject, pickProject, removeRecentProject } from "@/lib/projects"
import { requestProjectChange } from "@/lib/project-change-guard"

export function useProjectActions() {
  const router = useRouter()
  const fail = (error: unknown) => void toast.error(String(error))

  const openPath = async (path: string) => {
    const release = await requestProjectChange()
    if (!release) return
    try {
      await openProject(path)
      await router.invalidate()
      await router.navigate({ to: "/workflows" })
    } catch (error) {
      fail(error)
    } finally {
      release()
    }
  }

  return {
    openPath,
    pickAndOpen: async () => {
      const release = await requestProjectChange()
      if (!release) return
      try {
        const state = await pickProject()
        if (state) {
          await router.invalidate()
          await router.navigate({ to: "/workflows" })
        }
      } catch (error) {
        fail(error)
      } finally {
        release()
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
