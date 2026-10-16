import { useState } from "react"
import { AppSidebar, sections, type SectionId } from "@/components/app-sidebar"
import { SiteHeader } from "@/components/site-header"
import { TooltipProvider } from "@/components/ui/tooltip"

export default function App() {
  const [activeSection, setActiveSection] = useState<SectionId>("workflows")
  const section = sections.find(({ id }) => id === activeSection)!
  return (
    <TooltipProvider>
      <div className="flex h-full w-full overflow-hidden bg-background">
        <AppSidebar activeSection={activeSection} onSelect={setActiveSection} />
        <main className="min-h-0 min-w-0 flex-1 overflow-y-auto overscroll-none px-8 pt-6 pb-8">
          <SiteHeader title={section.label} />
        </main>
      </div>
    </TooltipProvider>
  )
}
