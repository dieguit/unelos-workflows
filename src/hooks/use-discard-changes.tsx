import { useState } from "react"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog"

/** Shared Stay / Discard prompt for inspector drafts and temporary Markdown buffers. */
export function useDiscardChanges() {
  const [pending, setPending] = useState<{
    message: string
    resolve: (discard: boolean) => void
  }>()
  const ask = (message = "Discard unsaved changes?") =>
    new Promise<boolean>((resolve) => setPending({ message, resolve }))
  const finish = (discard: boolean) => {
    pending?.resolve(discard)
    setPending(undefined)
  }
  return {
    ask,
    dialog: (
      <Dialog
        open={!!pending}
        onOpenChange={(open) => {
          if (!open) finish(false)
        }}
      >
        <DialogContent showCloseButton={false}>
          <DialogTitle>Unsaved changes</DialogTitle>
          <DialogDescription>{pending?.message}</DialogDescription>
          <DialogFooter>
            <Button variant="outline" onClick={() => finish(false)}>
              Stay
            </Button>
            <Button variant="destructive" onClick={() => finish(true)}>
              Discard changes
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    ),
  }
}
