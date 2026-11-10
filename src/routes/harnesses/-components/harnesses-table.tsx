import { Fragment } from "react"
import { Loader2Icon, MoreHorizontalIcon, SearchXIcon } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { TableCell, TableRow } from "@/components/ui/table"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { EmptyState } from "@/components/empty-state"
import {
  ListBody,
  ListCell,
  ListHead,
  ListHeader,
  ListPrimary,
  ListRow,
  ListTable,
} from "@/components/list-table"
import type { Harness } from "@/lib/harnesses"
import { cn } from "@/lib/utils"
import { filterHarnesses } from "../-lib/harnesses-list"
import type { HarnessAction } from "../-hooks/use-harnesses"

type Props = {
  harnesses: Harness[]
  query: string
  onClear: () => void
  pending: Record<string, HarnessAction>
  errors: Record<string, string>
  onAction: (harness: Harness, action: HarnessAction) => void
}

const Spinner = () => (
  <Loader2Icon aria-hidden="true" className="animate-spin" />
)

function Status({ h }: { h: Harness }) {
  if (h.installed)
    return (
      <span className="flex flex-wrap items-center gap-1.5">
        <Badge className="bg-success-muted text-success">Installed</Badge>
        {h.updateAvailable ? (
          <Badge className="bg-warning-muted text-warning">
            Update available
          </Badge>
        ) : null}
      </span>
    )
  if (h.detected) return <Badge variant="outline">Detected</Badge>
  return <span className="text-muted-foreground">Not detected</span>
}

function Actions({
  h,
  pending,
  onAction,
}: {
  h: Harness
  pending?: HarnessAction
  onAction: Props["onAction"]
}) {
  if (!h.installable)
    return (
      <span className="text-sm text-muted-foreground">
        Integration coming soon
      </span>
    )
  if (!h.installed && !h.detected)
    return (
      <Tooltip>
        <TooltipTrigger
          render={<span tabIndex={0} className="inline-flex rounded-lg" />}
        >
          <Button size="sm" variant="outline" disabled>
            Install
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          Install {h.name} first — not found on PATH
        </TooltipContent>
      </Tooltip>
    )
  if (!h.installed)
    return (
      <Button
        size="sm"
        disabled={!!pending}
        onClick={() => onAction(h, "install")}
      >
        {pending ? <Spinner /> : null}
        {pending ? "Installing…" : "Install"}
      </Button>
    )
  return (
    <span className="inline-flex items-center gap-1">
      {h.updateAvailable ? (
        <Button
          size="sm"
          disabled={!!pending}
          onClick={() => onAction(h, "update")}
        >
          {pending === "update" ? <Spinner /> : null}
          {pending === "update" ? "Updating…" : "Update"}
        </Button>
      ) : null}
      <DropdownMenu>
        <DropdownMenuTrigger
          disabled={!!pending}
          aria-label={`More actions for ${h.name}`}
          render={<Button size="icon-sm" variant="ghost" />}
        >
          {pending && pending !== "update" ? (
            <Spinner />
          ) : (
            <MoreHorizontalIcon aria-hidden="true" />
          )}
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-36">
          <DropdownMenuItem onClick={() => onAction(h, "reinstall")}>
            Reinstall
          </DropdownMenuItem>
          <DropdownMenuItem
            variant="destructive"
            onClick={() => onAction(h, "uninstall")}
          >
            Uninstall
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </span>
  )
}

export function HarnessesTable({
  harnesses,
  query,
  onClear,
  pending,
  errors,
  onAction,
}: Props) {
  const shown = filterHarnesses(harnesses, query)
  if (shown.length === 0)
    return (
      <EmptyState
        icon={SearchXIcon}
        title={`No harnesses match “${query.trim()}”`}
        action={
          <Button variant="outline" size="sm" onClick={onClear}>
            Clear search
          </Button>
        }
      />
    )
  return (
    <ListTable>
      <ListHeader>
        <ListHead>Harness</ListHead>
        <ListHead className="w-56">Status</ListHead>
        <ListHead className="w-32">Version</ListHead>
        <ListHead className="w-48">
          <span className="sr-only">Actions</span>
        </ListHead>
      </ListHeader>
      <ListBody>
        {shown.map((h) => {
          const dim = !h.installable && !h.detected
          const error = errors[h.id]
          return (
            <Fragment key={h.id}>
              <ListRow
                data-harness={h.id}
                className={cn(dim && "opacity-55", error && "border-b-0")}
              >
                <ListCell>
                  <ListPrimary title={h.name} subtitle={h.id} />
                </ListCell>
                <ListCell>
                  <Status h={h} />
                </ListCell>
                <ListCell className="font-mono text-sm text-foreground-dim">
                  {h.version ?? "—"}
                </ListCell>
                <ListCell className="text-right">
                  <Actions h={h} pending={pending[h.id]} onAction={onAction} />
                </ListCell>
              </ListRow>
              {error ? (
                <TableRow className="hover:bg-transparent">
                  <TableCell
                    colSpan={4}
                    role="alert"
                    className="px-3 pt-0 pb-2.5 text-sm [overflow-wrap:anywhere] whitespace-normal text-destructive"
                  >
                    {error}
                  </TableCell>
                </TableRow>
              ) : null}
            </Fragment>
          )
        })}
      </ListBody>
    </ListTable>
  )
}
