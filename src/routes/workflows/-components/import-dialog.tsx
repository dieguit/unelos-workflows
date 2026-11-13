import { open as pickFolder } from "@tauri-apps/plugin-dialog"
import {
  ArrowLeftIcon,
  ChevronRightIcon,
  CircleAlertIcon,
  CircleXIcon,
  LoaderCircleIcon,
  RotateCcwIcon,
  XIcon,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { cn } from "@/lib/utils"
import {
  useWorkflowImport,
  type ImportStep,
} from "../-hooks/use-workflow-import"
import { splitError } from "../-lib/import-plan"
import { ImportNamesStep } from "./import-names-step"
import { ImportReviewStep } from "./import-review-step"
import { ImportSourceStep } from "./import-source-step"

type Imported = Parameters<Parameters<typeof useWorkflowImport>[1]>[0]

const TOP = "clamp(24px,14vh,128px)"
const labels: Record<ImportStep, string> = {
  source: "Source",
  review: "Review",
  names: "Names",
}

export function ImportDialog({
  root,
  onClose,
  onImported,
}: {
  root: string
  onClose: () => void
  onImported: (result: Imported) => void
}) {
  const imp = useWorkflowImport(root, onImported)
  const { step, plan, importing, failure } = imp
  const needsNames = !!plan?.conflicts.length
  const trail = (["source", "review", "names"] as const).slice(
    0,
    ["source", "review", "names"].indexOf(step) + 1
  )
  const failed = failure ? splitError(failure) : null
  const errorCount = plan?.conflicts.filter((c) => c.error).length ?? 0
  const trusted = !plan?.mcps.length || imp.trust

  const primary =
    step === "source"
      ? {
          label: "Continue",
          disabled: !imp.source.trim() || imp.preparing,
          run: () => void imp.prepare(),
        }
      : step === "review"
        ? {
            label: needsNames ? "Continue" : "Import",
            disabled: !trusted,
            run: needsNames
              ? () => imp.setStep("names")
              : () => void imp.commit(),
          }
        : {
            label: "Import",
            disabled: errorCount > 0 || imp.planning,
            run: () => void imp.commit(),
          }
  const retry = failed && step !== "source"
  const run = () =>
    (retry || step !== "source") &&
    !primary.disabled &&
    !importing &&
    primary.run()

  function close() {
    if (importing) return
    imp.cancel()
    onClose()
  }

  async function browse() {
    const picked = await pickFolder({ directory: true }).catch(() => null)
    if (typeof picked === "string") imp.setSource(picked)
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && close()}
      disablePointerDismissal
    >
      <DialogContent
        showCloseButton={false}
        style={{ top: TOP, maxHeight: `calc(100vh - ${TOP} - 24px)` }}
        className="flex w-[640px] -translate-y-0 flex-col gap-0 p-0 sm:max-w-[640px]"
        onKeyDown={(e) => {
          if (e.key === "Enter" && !(e.target instanceof HTMLButtonElement))
            run()
        }}
      >
        <DialogHeader className="gap-1 px-6 pt-5 pb-3">
          <ol
            aria-label="Progress"
            className="flex items-center gap-1.5 text-xs font-medium tracking-[0.6px] uppercase"
          >
            {trail.map((key, i) => (
              <li
                key={key}
                aria-current={key === step ? "step" : undefined}
                className={cn(
                  "flex items-center gap-1.5",
                  key === step ? "text-foreground" : "text-muted-foreground"
                )}
              >
                {i > 0 ? (
                  <ChevronRightIcon aria-hidden className="size-3" />
                ) : null}
                {labels[key]}
              </li>
            ))}
          </ol>
          <DialogTitle className="text-xl leading-7 font-semibold">
            Import workflow
          </DialogTitle>
          <DialogDescription className="sr-only">
            Bring a workflow from a folder or git repository into this project.
          </DialogDescription>
          <DialogClose
            render={
              <Button
                variant="ghost"
                size="icon-sm"
                className="absolute top-4 right-4"
                disabled={importing}
              />
            }
          >
            <XIcon />
            <span className="sr-only">Close</span>
          </DialogClose>
        </DialogHeader>

        {failed && (
          <div className="px-6 pb-3">
            <div
              role="alert"
              className="flex gap-2 rounded-lg border border-destructive/60 bg-destructive-muted p-3 text-sm"
            >
              <CircleXIcon
                aria-hidden
                className="mt-0.5 size-4 shrink-0 text-destructive"
              />
              <div className="flex min-w-0 flex-col gap-1">
                <p className="font-medium">
                  Import failed. Nothing was added to your project.
                </p>
                <p className="font-mono text-xs break-words">{failed.title}</p>
                {failed.hint ? (
                  <p className="text-xs text-foreground-dim">{failed.hint}</p>
                ) : null}
              </div>
            </div>
          </div>
        )}
        <div
          inert={importing}
          className="flex min-h-0 flex-1 flex-col overflow-y-auto px-6 pb-4"
        >
          {step === "source" && (
            <ImportSourceStep
              source={imp.source}
              onChange={imp.setSource}
              onSubmit={() => void imp.prepare()}
              onBrowse={() => void browse()}
              loading={imp.preparing}
              error={imp.error}
            />
          )}
          {step === "review" && plan && (
            <ImportReviewStep
              plan={plan}
              trust={imp.trust}
              onTrust={imp.setTrust}
            />
          )}
          {step === "names" && plan && (
            <ImportNamesStep
              plan={plan}
              renames={imp.renames}
              notice={imp.notice}
              onRename={imp.rename}
            />
          )}
        </div>

        <DialogFooter className="mx-0 mb-0 items-center rounded-b-xl px-6">
          {step !== "source" && (
            <Button variant="ghost" onClick={imp.back} disabled={importing}>
              <ArrowLeftIcon /> Back
            </Button>
          )}
          <p
            aria-live="polite"
            className="flex items-center gap-1.5 text-sm text-foreground-dim sm:mr-auto"
          >
            {step === "review" && !trusted ? (
              <>
                <CircleAlertIcon aria-hidden className="size-4" /> Confirm you
                trust the MCP servers to continue
              </>
            ) : step === "names" && errorCount > 0 ? (
              <>
                <CircleAlertIcon
                  aria-hidden
                  className="size-4 text-destructive"
                />
                {errorCount} {errorCount === 1 ? "name needs" : "names need"}{" "}
                attention
              </>
            ) : null}
          </p>
          <Button variant="outline" onClick={close} disabled={importing}>
            Cancel
          </Button>
          {importing ? (
            <Button disabled>
              <LoaderCircleIcon className="animate-spin" /> Importing…
            </Button>
          ) : failed ? (
            <Button onClick={() => void imp.commit()}>
              <RotateCcwIcon /> Retry
            </Button>
          ) : step === "source" ? (
            <Button
              type="submit"
              form="import-source-form"
              disabled={primary.disabled}
            >
              {imp.preparing ? (
                <LoaderCircleIcon className="animate-spin" />
              ) : null}
              Continue
            </Button>
          ) : (
            <Button
              autoFocus={step === "review" && !plan?.mcps.length}
              onClick={primary.run}
              disabled={primary.disabled}
            >
              {primary.label}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
