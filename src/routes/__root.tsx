import { createRootRoute, Outlet } from "@tanstack/react-router"
import { AppSidebar } from "./-components/app-sidebar"
import { TooltipProvider } from "@/components/ui/tooltip"

export const Route = createRootRoute({
  component: () => (
    <TooltipProvider>
      <div className="flex h-full w-full overflow-hidden bg-background">
        <AppSidebar />
        <main className="min-h-0 min-w-0 flex-1 overflow-y-auto overscroll-none px-8 pt-6 pb-8">
          <Outlet />
        </main>
      </div>
    </TooltipProvider>
  ),
})
