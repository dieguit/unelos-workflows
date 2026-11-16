import { useRef, useState } from "react"
import { useRouter } from "@tanstack/react-router"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import {
  Dialog, DialogClose, DialogContent, DialogDescription,
  DialogFooter, DialogTitle, DialogTrigger,
} from "@/components/ui/dialog"
import { uninstallProject } from "@/lib/projects"
import { requestProjectChange } from "@/lib/project-change-guard"

export function UninstallProject({ root }: { root: string }) {
  const router = useRouter()
  const [open, setOpen] = useState(false)
  const [busy, setBusy] = useState(false)
  const submitting = useRef(false)

  const uninstall = async () => {
    if (submitting.current) return
    submitting.current = true
    setBusy(true)
    let release: (() => void) | null = null
    try {
      release = await requestProjectChange()
      if (!release) return
      await uninstallProject(root)
      // Leave Settings before invalidating, so deleted-project loaders never rerun.
      await router.navigate({ to: "/", search: { picker: true } })
      await router.invalidate()
    } catch (error) {
      toast.error(String(error))
    } finally {
      release?.()
      submitting.current = false
      setBusy(false)
    }
  }

  return (
    <section className="flex max-w-2xl flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
      <Dialog open={open} onOpenChange={(next) => {
        if (!submitting.current) setOpen(next)
      }}>
        <DialogTrigger render={<Button variant="destructive" disabled={busy} />}>
          Uninstall Unelos from this project
        </DialogTrigger>
        <DialogContent>
          <DialogTitle>Uninstall Unelos from this project?</DialogTitle>
          <p className="break-all font-mono text-sm">{root}</p>
          <DialogDescription>
            Delete .unelos (including workflows and run history) and generated Unelos agents? Your other project files and global integrations will remain.
          </DialogDescription>
          <DialogFooter>
            <DialogClose render={<Button variant="outline" disabled={busy} />}>Cancel</DialogClose>
            <Button variant="destructive" disabled={busy} onClick={uninstall}>
              {busy ? "Uninstalling…" : "Uninstall"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  )
}
