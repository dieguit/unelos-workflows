import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import { useEffect, useRef, useState, type ReactNode } from "react"
import { useBlocker, useRouter } from "@tanstack/react-router"
import { PlusIcon } from "lucide-react"
import { useStore } from "@tanstack/react-form"
import { BodyEditor } from "@/components/body-editor"
import { Button } from "@/components/ui/button"
import { useDiscardChanges } from "@/hooks/use-discard-changes"
import { useProjectDiscardGuard } from "@/hooks/use-project-discard-guard"
import { applyBodyDraft } from "@/lib/apply-body-draft"
import { ListSearch } from "@/components/list-search"
import { ListActions } from "@/components/list-actions"
import type { SkillSummary, TaskDocument, TaskSummary } from "@/lib/projects"
import { TaskInspector } from "@/components/task-inspector"
import { TasksList, filterTasks } from "@/components/tasks-list"
import { emptyTask, useTaskForm } from "@/hooks/use-task-form"
import { taskSchema } from "@/lib/task-schema"
import { outcomeRenames } from "@/lib/outcome-renames"

export type TasksApi = {
  get: (id: string) => Promise<TaskDocument>
  save: (originalId: string | null, task: TaskDocument, renames: Record<string, string>) => Promise<TaskDocument>
  duplicate: (id: string, newId: string) => Promise<void>
  remove: (id: string) => Promise<void>
  /** Project scope only. */
  makeGlobal?: (id: string) => Promise<void>
}

/** List + inspector for a task store: the open project's tasks or the machine-wide global ones. */
export function TasksPage({
  tasks,
  initial,
  api,
  fetchSkills,
  scope,
  header: Header,
}: {
  tasks: TaskSummary[]
  /** Task to open on mount (from the URL). */
  initial?: string
  api: TasksApi
  fetchSkills: (() => Promise<SkillSummary[]>) | null
  scope: { path: string; label: string }
  header: (props: { action?: ReactNode }) => ReactNode
}) {
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)
  const router = useRouter()
  // undefined = closed, null = new draft, string = existing identifier.
  const [selected, setSelected] = useState<string | null | undefined>(initial)
  const [loaded, setLoaded] = useState(false)
  const [baseline, setBaseline] = useState<TaskDocument>(emptyTask)
  const originals = useRef<(string | null)[]>([])
  const [loadError, setLoadError] = useState<string>()
  const [saveError, setSaveError] = useState<string>()
  const [editor, setEditor] = useState<string | null>(null)
  const discard = useDiscardChanges()
  const form = useTaskForm(
    baseline,
    async (updated, closeEditor) => {
      setSaveError(undefined)
      try {
        const saved = await api.save(selected ?? null, updated, outcomeRenames(originals.current, updated.outcomes))
        originals.current = [...saved.outcomes]
        setBaseline(saved)
        setSelected(saved.id)
        if (closeEditor) setEditor(null)
        void router.invalidate()
        return saved
      } catch (error) {
        setSaveError(String(error))
      }
    },
    () =>
      setSaveError(
        "Check the highlighted fields. Enter a Name (id) and named, unique outcomes before saving; Cancel instructions editing to return. Your text is kept."
      )
  )
  const draft = useStore(form.store, (s) => s.values)
  const saving = useStore(form.store, (s) => s.isSubmitting)
  const visualSaving = useDelayedFlag(saving)
  const isDefaultValue = useStore(form.store, (s) => s.isDefaultValue)
  const request = useRef(0)
  const dirty = loaded && !isDefaultValue
  const editorDirty = editor !== null && editor !== draft?.instructions
  const askDiscard = discard.ask
  const projectChangeApproved = useProjectDiscardGuard(saving, dirty || editorDirty, askDiscard)
  useBlocker({
    shouldBlockFn: async () =>
      !projectChangeApproved.current && (saving || ((dirty || editorDirty) && !(await askDiscard()))),
    enableBeforeUnload: dirty || editorDirty || saving,
  })

  // Callers reset loaded/loadError first; state is only set once the fetch settles.
  const load = (id: string) => {
    const token = ++request.current
    api.get(id).then(
      (doc) => {
        if (token !== request.current) return
        form.reset(doc)
        originals.current = [...doc.outcomes]
        setBaseline(doc)
        setLoaded(true)
      },
      (error) => {
        if (token === request.current) setLoadError(String(error))
      }
    )
  }
  // Mount only: the search param picks the first task to open.
  useEffect(() => {
    if (initial) load(initial)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])
  const select = async (id: string | null | undefined) => {
    if (saving || id === selected) return
    if ((dirty || editorDirty) && !(await askDiscard())) return
    ++request.current
    setSelected(id)
    setEditor(null)
    setSaveError(undefined)
    setLoadError(undefined)
    setLoaded(false)
    form.reset(emptyTask)
    originals.current = []
    setBaseline(emptyTask)
    if (typeof id === "string") load(id)
    else if (id === null) setLoaded(true)
  }

  const closeIf = (id: string) => {
    if (selected !== id) return
    ++request.current
    setSelected(undefined)
    setEditor(null)
    setLoaded(false)
    form.reset(emptyTask)
    originals.current = []
    setBaseline(emptyTask)
  }
  const actions = (id: string, variant?: "footer") => (
    <ListActions
      variant={variant}
      kind="Task"
      name={id}
      nameSchema={taskSchema.shape.id}
      disabled={visualSaving}
      beforeRemove={async () =>
        selected !== id ||
        (!saving && (!(dirty || editorDirty) || (await askDiscard())))
      }
      duplicate={async (newId) => {
        await api.duplicate(id, newId)
        void router.invalidate()
      }}
      remove={async () => {
        await api.remove(id)
        closeIf(id)
        void router.invalidate()
      }}
      makeGlobal={
        api.makeGlobal &&
        (async () => {
          await api.makeGlobal!(id)
          closeIf(id)
          void router.invalidate()
        })
      }
      makeGlobalSkills={async () =>
        (await api.get(id)).skills.filter((s) => !s.startsWith("global/"))
      }
      openGlobal={() =>
        void router.navigate({ to: "/global-tasks", search: { task: id } })
      }
    />
  )

  return (
    <div inert={saving} className="-mx-8 -mt-6 -mb-8 flex h-[calc(100%+3.5rem)]">
      <div className="min-w-0 flex-1 overflow-y-auto px-8 pt-6 pb-8">
        {editor !== null && draft ? (
          <div className="flex h-full flex-col gap-4">
            <BodyEditor
              title={draft.id || "New Task"}
              label="Instructions"
              body={editor}
              onChange={setEditor}
              onCancel={() => {
                void (async () => {
                  if (
                    !editorDirty ||
                    (await askDiscard(
                      "Discard unsaved changes to instructions? The Task draft will be kept."
                    ))
                  )
                    setEditor(null)
                })()
              }}
              onSave={async () => {
                applyBodyDraft(editor, (body) => form.setFieldValue("instructions", body), () => setEditor(null))
              }}
              onBlur={() => {
                void form.validateField("instructions", "blur")
              }}
            />
          </div>
        ) : (
          <div className="flex flex-col gap-4">
            <Header
              action={
                <Button onClick={() => void select(null)} disabled={visualSaving}>
                  <PlusIcon aria-hidden="true" />
                  New Task
                </Button>
              }
            />
            {tasks.length > 0 ? (
              <ListSearch
                noun="task"
                value={query}
                onChange={setQuery}
                shown={filterTasks(tasks, query).length}
                total={tasks.length}
                inputRef={search}
              />
            ) : null}
            <TasksList
              tasks={tasks}
              selected={selected ?? undefined}
              onSelect={(id) => void select(id)}
              onCreate={() => void select(null)}
              actions={(id) => actions(id)}
              query={query}
              onClear={() => {
                setQuery("")
                search.current?.focus()
              }}
            />
          </div>
        )}
      </div>
      {selected !== undefined ? (
        <TaskInspector
          key={selected ?? "\0new"}
          fetchSkills={fetchSkills}
          scope={scope}
          originalId={selected}
          form={form}
          footer={typeof selected === "string" && loaded ? actions(selected, "footer") : undefined}
          onOutcomeAdd={() => originals.current.push(null)}
          onOutcomeRemove={(i) => originals.current.splice(i, 1)}
          draft={loaded ? draft : undefined}
          dirty={dirty}
          loading={!loaded && !loadError}
          loadError={loadError}
          saveError={editor !== null ? undefined : saveError}
          saving={saving}
          editing={editor !== null}
          onEdit={() => draft && setEditor(draft.instructions)}
          onClose={() => void select(undefined)}
          onRetry={() => {
            if (typeof selected !== "string") return
            setLoadError(undefined)
            load(selected)
          }}
        />
      ) : null}
      {discard.dialog}
    </div>
  )
}

/** Load failure for a tasks page, with Retry. */
export function TasksError({
  error,
  header: Header,
}: {
  error: unknown
  header: (props: { action?: ReactNode }) => ReactNode
}) {
  const router = useRouter()
  return (
    <div className="flex flex-col gap-6">
      <Header />
      <div className="flex flex-col items-start gap-3 rounded-lg border border-destructive/40 p-4">
        <p className="text-sm [overflow-wrap:anywhere]">
          Couldn’t load tasks:{" "}
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
