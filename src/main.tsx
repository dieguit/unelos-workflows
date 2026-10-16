import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { createRouter, RouterProvider } from "@tanstack/react-router"

import "./index.css"
import { routeTree } from "./routeTree.gen"
import { ThemeProvider } from "@/components/theme-provider.tsx"
import { ExternalLinkGuard } from "./components/external-link-guard.tsx"
import { DebugPanel } from "./components/debug-panel.tsx"
import { isTauri } from "./lib/tauri"

const router = createRouter({ routeTree })

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router
  }
}

if (import.meta.env.DEV && !isTauri()) {
  ;(await import("./lib/dev-ipc-mock")).installDevIpcMock()
}
window.addEventListener("focus", () => void router.invalidate())

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      <ExternalLinkGuard />
      {import.meta.env.DEV ? <DebugPanel /> : null}
      <div data-ui-scroll-container><RouterProvider router={router} /></div>
    </ThemeProvider>
  </StrictMode>
)
