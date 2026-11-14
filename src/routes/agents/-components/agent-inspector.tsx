import { BusyContent } from "@/components/busy-content"
import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import { useCallback, type ReactNode } from "react"
import { McpPicker } from "./mcp-picker"
import { Link } from "@tanstack/react-router"
import { PanelRightCloseIcon, PencilIcon, XIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  Field,
  FieldError,
  FieldGroup,
  FieldLabel,
  FieldTitle,
} from "@/components/ui/field"
import { listAllSkills } from "@/hooks/skill-fetchers"
import { listGlobalMcps, listMcps, type AgentDocument, type Reference } from "@/lib/projects"
import { UsedByRow } from "@/components/used-by"
import type { AgentForm } from "../-hooks/use-agent-form"
import { useLazy } from "@/hooks/use-lazy"
import { AgentModelPicker } from "./model-picker"
import { SkillPicker } from "@/components/skill-picker"
import { SkillLink } from "@/components/skill-link"

export function AgentInspector({
  root,
  usedBy,
  originalId,
  form,
  draft,
  dirty,
  loading,
  loadError,
  saveError,
  saving,
  editing,
  onEdit,
  onClose,
  onRetry,
  footer,
}: {
  footer?: ReactNode
  root: string
  usedBy: Reference[] | null
  originalId: string | null
  form: AgentForm
  draft?: AgentDocument
  dirty: boolean
  loading: boolean
  loadError?: string
  saveError?: string
  saving: boolean
  editing: boolean
  onEdit: () => void
  onClose: () => void
  onRetry: () => void
}) {
  const visualSaving = useDelayedFlag(saving)
  const fetchSkills = useCallback(() => listAllSkills(root), [root])
  const skills = useLazy(fetchSkills)
  const fetchMcps = useCallback(async () => {
    const [project, global] = await Promise.all([listMcps(root), listGlobalMcps()])
    return [...project, ...global.map((m) => ({ ...m, name: `global/${m.name}` }))]
  }, [root])
  const mcps = useLazy(fetchMcps)
  return (
    <aside inert={saving}
      aria-label={originalId ? `Agent ${originalId}` : "New Agent"}
      className="flex w-[380px] max-w-full shrink-0 flex-col border-l bg-background"
    >
      <header className="flex min-h-16 shrink-0 items-center gap-2 px-4 py-3">
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-base font-semibold">
            {originalId ?? "New Agent"}
          </h2>
          <p className="truncate font-mono text-xs text-muted-foreground">
            {originalId ? `agents/${originalId}` : "Project Agent"}
          </p>
          {dirty ? (
            <p role="status" className="mt-1 text-xs text-warning">
              Unsaved changes
            </p>
          ) : null}
        </div>
        <Button
          size="sm"
          type="submit"
          form="agent-form"
          disabled={!dirty || visualSaving || editing}
        >
          <BusyContent busy={visualSaving}>Save</BusyContent>
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Close panel"
          title="Close panel"
          disabled={visualSaving}
          onClick={onClose}
        >
          <PanelRightCloseIcon aria-hidden="true" />
        </Button>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {loading ? (
          <p role="status" className="text-sm text-muted-foreground">
            Loading Agent…
          </p>
        ) : null}
        {loadError ? (
          <div role="alert" className="flex flex-col items-start gap-3 text-sm">
            <p className="[overflow-wrap:anywhere] text-destructive">
              Could not load Agent: {loadError}
            </p>
            <Button variant="outline" size="sm" onClick={onRetry}>
              Retry
            </Button>
          </div>
        ) : null}
        {draft ? (
          <form
            id="agent-form"
            noValidate
            onSubmit={(e) => {
              e.preventDefault()
              if (dirty && !saving && !editing) void form.handleSubmit()
            }}
          >
            <fieldset
              disabled={visualSaving || editing}
              className="min-w-0 disabled:opacity-60"
            >
              <FieldGroup className="gap-4">
                <form.Field name="id">
                  {(field) => {
                    const invalid =
                      (field.state.meta.isBlurred &&
                        !field.state.meta.isValid) ||
                      (form.state.submissionAttempts > 0 &&
                        !field.state.meta.isValid)
                    return !originalId ? (
                      <Field data-invalid={invalid} className="grid grid-cols-[76px_minmax(0,1fr)] items-center gap-x-2 [&>label]:text-foreground-dim [&>div]:col-start-2">
                        <FieldLabel htmlFor="agent-id">Name (id)</FieldLabel>
                        <Input
                          id="agent-id"
                          name={field.name}
                          value={field.state.value}
                          required
                          autoFocus
                          spellCheck={false}
                          autoComplete="off"
                          placeholder="e.g. reviewer"
                          onBlur={field.handleBlur}
                          onChange={(e) => {
                            field.handleChange(e.target.value)
                            if (field.state.meta.isBlurred) field.handleBlur()
                          }}
                          aria-invalid={invalid}
                          aria-describedby={
                            invalid ? "agent-id-error" : undefined
                          }
                          className="h-9 font-mono font-normal"
                        />
                        <div>
                          {invalid ? (
                            <FieldError
                              id="agent-id-error"
                              errors={field.state.meta.errors}
                            />
                          ) : null}
                        </div>
                      </Field>
                    ) : null
                  }}
                </form.Field>
                <form.Field name="instructions">
                  {(field) => (
                    <Field>
                      <div className="flex items-center justify-between gap-2">
                        <FieldTitle>Instructions</FieldTitle>
                        <Button variant="outline" size="sm" onClick={onEdit}>
                          <PencilIcon aria-hidden="true" />
                          Edit instructions
                        </Button>
                      </div>
                      <pre className="line-clamp-5 rounded-lg border bg-card p-3 font-mono text-xs leading-relaxed [overflow-wrap:anywhere] whitespace-pre-wrap text-foreground-dim">
                        {field.state.value.trim() || "Empty"}
                      </pre>
                    </Field>
                  )}
                </form.Field>
                <form.Field name="skills" mode="array">
                  {(field) => (
                    <Field onBlur={field.handleBlur}>
                      <FieldTitle>Skills</FieldTitle>
                      {field.state.value.length ? (
                        <ul className="flex flex-wrap gap-1.5">
                          {field.state.value.map((skill, i) => (
                            <li
                              key={`${i}-${skill}`}
                              className="flex h-7 min-w-0 items-center gap-1 rounded-md border pr-0.5 pl-1.5 font-mono text-xs"
                            >
                              <SkillLink
                                skill={skill}
                                className="min-w-0 flex-1 truncate text-sm"
                              />
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                aria-label={`Remove ${skill}`}
                                onClick={() => field.removeValue(i)}
                              >
                                <XIcon aria-hidden="true" />
                              </Button>
                            </li>
                          ))}
                        </ul>
                      ) : (
                        <p className="text-sm text-muted-foreground">
                          Not configured
                        </p>
                      )}
                      <SkillPicker
                        skills={skills.data}
                        error={skills.error}
                        taken={field.state.value}
                        onOpen={skills.load}
                        onPick={(skill) => field.pushValue(skill)}
                      />
                    </Field>
                  )}
                </form.Field>
                <form.Field name="model">
                  {(field) => (
                    <Field onBlur={field.handleBlur} className="grid grid-cols-[76px_minmax(0,1fr)] items-center gap-x-2 [&>label]:text-foreground-dim">
                      <FieldLabel htmlFor="agent-model">Model</FieldLabel>
                      <AgentModelPicker
                        value={field.state.value}
                        onChange={field.handleChange}
                      />
                    </Field>
                  )}
                </form.Field>
                <form.Field name="mcps" mode="array">
                  {(field) => (
                    <Field onBlur={field.handleBlur}>
                      <FieldTitle>MCPs</FieldTitle>
                      {field.state.value.length ? (
                        <ul className="flex flex-wrap gap-1.5">
                          {field.state.value.map((mcp, i) => (
                            <li
                              key={`${i}-${mcp}`}
                              className="flex h-7 min-w-0 items-center gap-1 rounded-md border pr-0.5 pl-1.5 font-mono text-xs"
                            >
                              <Link
                                to={mcp.startsWith("global/") ? "/global-mcps" : "/mcps"}
                                search={{ mcp: mcp.startsWith("global/") ? mcp.slice(7) : mcp }}
                                className="min-w-0 flex-1 truncate text-sm hover:underline"
                                title={mcp.startsWith("global/") ? "Open in Global MCPs" : "Open in MCPs"}
                              >
                                {mcp}
                              </Link>
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                aria-label={`Remove ${mcp}`}
                                onClick={() => field.removeValue(i)}
                              >
                                <XIcon aria-hidden="true" />
                              </Button>
                            </li>
                          ))}
                        </ul>
                      ) : (
                        <p className="text-sm text-muted-foreground">
                          Not configured
                        </p>
                      )}
                      <McpPicker
                        mcps={mcps.data}
                        error={mcps.error}
                        taken={field.state.value}
                        onOpen={mcps.load}
                        onPick={(mcp) => field.pushValue(mcp)}
                      />

                    </Field>
                  )}
                </form.Field>
              </FieldGroup>
            </fieldset>
            {originalId ? (
              <div className="mt-4">
                <UsedByRow refs={usedBy} />
              </div>
            ) : null}
            {saveError ? (
              <div
                role="alert"
                className="mt-5 flex flex-col items-start gap-2"
              >
                <p className="text-sm text-destructive">
                  Could not save Agent. Check project write access and try Save
                  again.
                </p>
                <p className="text-xs [overflow-wrap:anywhere] text-muted-foreground">
                  {saveError}
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  type="submit"
                  disabled={visualSaving || editing}
                >
                  Retry
                </Button>
              </div>
            ) : null}
          </form>
        ) : null}
      </div>
      {footer ? (
        <footer className="shrink-0 border-t p-3">{footer}</footer>
      ) : null}
    </aside>
  )
}
