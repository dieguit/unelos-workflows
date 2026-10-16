import { useEffect } from "react"
import { createRootRoute, Outlet } from "@tanstack/react-router"
import { Toaster } from "sonner"
import { AppSidebar } from "./-components/app-sidebar"
import { TooltipProvider } from "@/components/ui/tooltip"
import { useTheme } from "@/components/theme-provider"
import { useProjectActions } from "@/hooks/use-project-actions"
import { getProjectState } from "@/lib/projects"

export const Route = createRootRoute({
  beforeLoad: async () => {
    const projectState = await getProjectState()
    const first = projectState.recents[0]
    return { projectState, project: first?.available ? first : null }
  },
  component: RootLayout,
})

function RootLayout() {
  const { theme } = useTheme()
  const { pickAndOpen } = useProjectActions()
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "o") {
        e.preventDefault()
        void pickAndOpen()
      }
    }
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  }, [pickAndOpen])
  return (
    <TooltipProvider>
      <div className="flex h-full w-full overflow-hidden bg-background">
        <AppSidebar />
        <main className="min-h-0 min-w-0 flex-1 overflow-y-auto overscroll-none px-8 pt-6 pb-8">
          <Outlet />
        </main>
      </div>
      <Toaster theme={theme} position="bottom-right" />
    </TooltipProvider>
  )
}
