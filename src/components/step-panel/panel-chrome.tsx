import type { ReactNode } from "react"
import { PanelRightCloseIcon, Trash2Icon } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import type { WorkflowStep } from "@/lib/projects"
import { humanize } from "@/lib/step-edit"

export function PanelHeader({ icon, title, subtitle, tag, onClose }: {
  icon: ReactNode
  title: string
  subtitle: string
  tag?: string
  onClose: () => void
}) {
  return (
    <header className="flex items-center gap-2.5 py-3 pr-2 pl-4">
      {icon}
      <div className="min-w-0 flex-1">
        <h2 className="truncate text-lg font-semibold">{title}</h2>
        <p className="truncate font-mono text-xs text-muted-foreground">{subtitle}</p>
      </div>
      {tag && (
        <span className="shrink-0 rounded-sm border px-1.5 py-0.5 text-xs text-muted-foreground">{tag}</span>
      )}
      <Button variant="ghost" size="icon-sm" aria-label="Close step" title="Close step" onClick={onClose}>
        <PanelRightCloseIcon aria-hidden="true" />
      </Button>
    </header>
  )
}

export function PanelTabs() {
  return (
    <div role="tablist" aria-label="Step views" className="flex gap-4.5 border-b px-4 text-sm">
      <button
        type="button"
        role="tab"
        id="step-tab-properties"
        aria-selected="true"
        aria-controls="step-tab-panel"
        className="-mb-px h-8.5 border-b-2 border-primary font-medium"
      >
        Properties
      </button>
      <Tooltip>
        <TooltipTrigger
          render={
            <button
              type="button"
              role="tab"
              aria-selected="false"
              aria-disabled="true"
              className="h-8.5 cursor-not-allowed text-muted-foreground"
            />
          }
        >
          Context preview
        </TooltipTrigger>
        <TooltipContent>Coming soon</TooltipContent>
      </Tooltip>
    </div>
  )
}

export function PanelBody({ disabled, footer, children }: { disabled: boolean; footer?: ReactNode; children: ReactNode }) {
  return (
    <div id="step-tab-panel" role="tabpanel" aria-labelledby="step-tab-properties" className="min-h-0 flex-1 overflow-y-auto">
      <fieldset disabled={disabled} className="flex min-w-0 flex-col gap-4 p-4 disabled:opacity-60">
        {children}
      </fieldset>
      {footer && <div className="border-t px-4 py-4">{footer}</div>}
    </div>
  )
}

export function RemoveStep({ step, disabled, onRemove }: { step: WorkflowStep; disabled: boolean; onRemove: () => void }) {
  return (
    <Dialog key={step.id}>
      <DialogTrigger render={<Button variant="destructive" size="sm" disabled={disabled} />}>
        <Trash2Icon aria-hidden="true" />
        Remove step
      </DialogTrigger>
      <DialogContent>
        <DialogTitle>Remove {humanize(step.id)}?</DialogTitle>
        <DialogDescription>
          {step.kind === "parallel"
            ? "Removes this step, its tasks and its references. You may need to fix the workflow before saving."
            : "Removes this step and its references. You may need to fix the workflow before saving. The reusable task is kept."}
        </DialogDescription>
        <DialogFooter>
          <DialogClose render={<Button variant="outline" />}>Cancel</DialogClose>
          <Button variant="destructive" disabled={disabled} onClick={onRemove}>
            Remove step
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
