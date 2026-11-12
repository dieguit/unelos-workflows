import { useState } from "react"
import { Trash2Icon } from "lucide-react"
import { toast } from "sonner"
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

export function DeleteRunButton({ name, onDelete }: { name: string; onDelete: () => Promise<void> }) {
  const [open, setOpen] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (busy) return
        setError(undefined)
        setOpen(next)
      }}
    >
      <DialogTrigger render={<Button variant="destructive" size="sm" />}>
        <Trash2Icon aria-hidden="true" /> Delete
      </DialogTrigger>
      <DialogContent>
        <DialogTitle>Delete run?</DialogTitle>
        <DialogDescription>Delete {name} and all its artifacts? This cannot be undone.</DialogDescription>
        {error && (
          <p role="alert" className="text-sm [overflow-wrap:anywhere] text-destructive">
            {error}
          </p>
        )}
        <DialogFooter>
          <DialogClose render={<Button variant="outline" disabled={busy} />}>Cancel</DialogClose>
          <Button
            variant="destructive"
            disabled={busy}
            onClick={() => {
              setBusy(true)
              setError(undefined)
              onDelete().then(
                () => toast.success("Run removed"),
                (err) => {
                  setError(String(err))
                  setBusy(false)
                }
              )
            }}
          >
            {busy ? "Deleting…" : "Delete"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
