import { useRef, useState, type ReactNode } from "react"
import { createFileRoute, useRouter } from "@tanstack/react-router"
import { Loader2Icon, RefreshCwIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { ListSearch } from "@/components/list-search"
import { PageHeader } from "@/components/page-header"
import { listHarnesses } from "@/lib/harnesses"
import { MapModelsDialog } from "./-components/map-models-dialog"
import { HarnessesTable } from "./-components/harnesses-table"
import { useHarnesses } from "./-hooks/use-harnesses"
import { filterHarnesses } from "./-lib/harnesses-list"

export const Route = createFileRoute("/harnesses/")({
  loader: () => listHarnesses(),
  component: HarnessesHome,
  pendingComponent: () => (
    <div className="flex flex-col gap-6">
      <Header />
      <p role="status">Scanning harnesses…</p>
    </div>
  ),
  errorComponent: HarnessesError,
})

function Header({ action }: { action?: ReactNode }) {
  return (
    <PageHeader
      title="Harnesses"
      description="Coding agents unelos can run on. Install adds the unelos integration to the harness's user config, so it works in every project."
      action={action}
    />
  )
}

function HarnessesHome() {
  const { harnesses, pending, errors, scanning, run, rescan, mapping, closeMapping } = useHarnesses(
    Route.useLoaderData()
  )
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)

  return (
    <div className="flex flex-col gap-4">
      <Header
        action={
          <Button
            variant="outline"
            size="sm"
            disabled={scanning}
            onClick={() => void rescan()}
          >
            {scanning ? (
              <Loader2Icon aria-hidden="true" className="animate-spin" />
            ) : (
              <RefreshCwIcon aria-hidden="true" />
            )}
            Rescan
          </Button>
        }
      />
      <ListSearch
        noun="harness"
        value={query}
        onChange={setQuery}
        shown={filterHarnesses(harnesses, query).length}
        total={harnesses.length}
        inputRef={search}
      />
      <HarnessesTable
        harnesses={harnesses}
        query={query}
        onClear={() => {
          setQuery("")
          search.current?.focus()
        }}
        pending={pending}
        errors={errors}
        onAction={(h, action) => void run(h, action)}
      />
      {mapping ? (
        <MapModelsDialog
          harness={mapping.harness}
          rows={mapping.rows}
          onClose={() => void closeMapping()}
        />
      ) : null}
    </div>
  )
}

function HarnessesError({ error }: { error: unknown }) {
  const router = useRouter()
  return (
    <div className="flex flex-col gap-6">
      <Header />
      <div className="flex flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
        <p className="text-sm [overflow-wrap:anywhere]">
          Couldn’t scan harnesses:{" "}
          {error instanceof Error ? error.message : String(error)}
        </p>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void router.invalidate()}
        >
          Retry
        </Button>
      </div>
    </div>
  )
}
