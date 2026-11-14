import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import { useEffect, useRef, useState, type ReactNode } from "react"
import { useBlocker, useRouter } from "@tanstack/react-router"
import { toast } from "sonner"
import { useStore } from "@tanstack/react-form"
import { PlusIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { ListSearch } from "@/components/list-search"
import { BodyEditor } from "@/components/body-editor"
import { ListActions } from "@/components/list-actions"
import type {
  SkillDocument,
  SkillSaveResult,
  SkillSummary,
} from "@/lib/projects"
import { SkillInspector } from "@/components/skill-inspector"
import { useSkillForm, type SkillDraft } from "@/hooks/use-skill-form"
import { useDiscardChanges } from "@/hooks/use-discard-changes"
import { useProjectDiscardGuard } from "@/hooks/use-project-discard-guard"
import { applyBodyDraft } from "@/lib/apply-body-draft"
import { SkillsList, filterSkills } from "@/components/skills-list"
import { useSkill } from "@/hooks/use-skill"
import type { UsedBy } from "@/lib/references"
import { skillSchema } from "@/lib/skill-schema"

type SkillsApi = {
  get: (id: string) => Promise<SkillDocument>
  save: (id: string, value: SkillDraft) => Promise<SkillSaveResult>
  duplicate: (id: string, newId: string) => Promise<unknown>
  remove: (id: string) => Promise<unknown>
  /** Project scope only. */
  makeGlobal?: (id: string) => Promise<unknown>
  create: (
    name: string,
    description: string,
    body: string
  ) => Promise<SkillDocument>
}

export function SkillsPage({
  skills,
  usedBy,
  id,
  scope,
  api,
  header: Header,
  navigate,
}: {
  skills: SkillSummary[]
  usedBy: UsedBy
  id: string | undefined
  scope: string
  api: SkillsApi
  header: (props: { action?: ReactNode }) => ReactNode
  navigate: (o: {
    search: { skill?: string }
    replace?: boolean
    ignoreBlocker?: boolean
  }) => unknown
}) {
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)
  const router = useRouter()
  const { doc, error, setDoc } = useSkill(scope, api.get, id)
  const [saveError, setSaveError] = useState<string>()
  const [creating, setCreating] = useState(false)
  useEffect(() => {
    if (id) setCreating(false)
  }, [id])
  const form = useSkillForm(
    doc
      ? { ...doc, description: doc.description ?? "" }
      : { name: "", description: "", body: "" },
    async (value, closeEditor) => {
      if (creating) {
        setSaveError(undefined)
        try {
          const created = await api.create(
            value.name,
            value.description,
            value.body
          )
          setDoc(created)
          setCreating(false)
          setEditor(null)
          void navigate({
            search: { skill: created.name },
            replace: true,
            ignoreBlocker: true,
          })
          void router.invalidate()
          return { ...created, description: created.description ?? "" }
        } catch (error) {
          setSaveError(String(error))
        }
        return
      }
      if (!doc) return
      setSaveError(undefined)
      try {
        const res = await api.save(id!, value)
        applySave(id!, res)
        if (closeEditor) setEditor(null)
        return { ...res.skill, description: res.skill.description ?? "" }
      } catch (error) {
        setSaveError(String(error))
      }
    }
  )
  const draft = useStore(form.store, (s) => s.values)
  const saving = useStore(form.store, (s) => s.isSubmitting)
  const visualSaving = useDelayedFlag(saving)
  const isDefaultValue = useStore(form.store, (s) => s.isDefaultValue)
  const discard = useDiscardChanges()
  useEffect(() => {
    form.reset(
      doc
        ? { ...doc, description: doc.description ?? "" }
        : { name: "", description: "", body: "" }
    )
    setSaveError(undefined)
  }, [doc, form])
  const newButton = (
    <Button onClick={() => void startCreate()} disabled={visualSaving}>
      <PlusIcon aria-hidden="true" />
      New skill
    </Button>
  )
  const dirty = (!!doc || creating) && !isDefaultValue
  const [editor, setEditor] = useState<{ id: string; body: string } | null>(
    null
  )
  useEffect(() => setEditor(null), [id])
  const current = creating ? "" : id
  const editing =
    editor && (doc || creating) && editor.id === current ? editor : null

  const startCreate = async () => {
    if (saving || creating) return
    if (dirty && !(await discard.ask("Discard unsaved changes to this Skill?")))
      return
    if (id) await navigate({ search: {}, ignoreBlocker: true })
    form.reset({ name: "", description: "", body: "" })
    setSaveError(undefined)
    setCreating(true)
  }

  const discardEdits = async () =>
    !editing ||
    editing.body === draft.body ||
    (await discard.ask(
      "Discard unsaved changes to SKILL.md? The Skill draft will be kept."
    ))

  const projectChangeApproved = useProjectDiscardGuard(saving, dirty || (!!editing && editing.body !== draft.body), discard.ask)
  useBlocker({
    shouldBlockFn: async () =>
      !projectChangeApproved.current && (saving ||
      ((dirty || (!!editing && editing.body !== doc?.body)) &&
        !(await discard.ask("Discard unsaved changes to this Skill?")))),
    enableBeforeUnload:
      dirty || (!!editing && editing.body !== doc?.body) || saving,
  })

  const applySave = (
    from: string,
    { skill, updatedReferences }: SkillSaveResult
  ) => {
    setDoc(skill)
    if (skill.name !== from) {
      setEditor((e) => (e?.id === from ? { ...e, id: skill.name } : e))
      void navigate({
        search: { skill: skill.name },
        replace: true,
        ignoreBlocker: true,
      })
    }
    void router.invalidate()
    const n = updatedReferences.length
    if (n > 0)
      toast.success(
        `Updated ${n} reference${n === 1 ? "" : "s"} to ${skill.name}`,
        {
          description: updatedReferences.join(", "),
        }
      )
  }

  const actions = (name: string, variant?: "footer") => (
    <ListActions
      variant={variant}
      kind="Skill"
      name={name}
      nameSchema={skillSchema.shape.name}
      disabled={visualSaving}
      beforeRemove={async () =>
        id !== name ||
        (!saving &&
          (!dirty ||
            (await discard.ask("Discard unsaved changes to this Skill?"))))
      }
      duplicate={async (newId) => {
        await api.duplicate(name, newId)
        void router.invalidate()
      }}
      remove={async () => {
        await api.remove(name)
        if (id === name) await navigate({ search: {}, ignoreBlocker: true })
        void router.invalidate()
      }}
      makeGlobal={
        api.makeGlobal &&
        (async () => {
          await api.makeGlobal!(name)
          if (id === name) await navigate({ search: {}, ignoreBlocker: true })
          void router.invalidate()
        })
      }
      openGlobal={() =>
        void router.navigate({ to: "/global-skills", search: { skill: name } })
      }
    />
  )

  return (
    // Cancel <main>'s padding so the inspector sits flush right at full height.
    <div inert={saving} className="-mx-8 -mt-6 -mb-8 flex h-[calc(100%+3.5rem)]">
      <div className="min-w-0 flex-1 overflow-y-auto px-8 pt-6 pb-8">
        {editing ? (
          <div className="flex h-full flex-col gap-4">
            <BodyEditor
              title={editing.id || "New skill"}
              label="SKILL.md"
              body={editing.body}
              onChange={(body) => setEditor({ ...editing, body })}
              onCancel={() => {
                void discardEdits().then((discard) => {
                  if (discard) setEditor(null)
                })
              }}
              onSave={async () => {
                applyBodyDraft(editing.body, (body) => form.setFieldValue("body", body), () => setEditor(null))
              }}
              onBlur={() => {
                void form.validateField("body", "blur")
              }}
            />
          </div>
        ) : (
          <div className="flex flex-col gap-4">
            <Header action={newButton} />
            {skills.length > 0 ? (
              <ListSearch
                noun="skill"
                value={query}
                onChange={setQuery}
                shown={filterSkills(skills, query).length}
                total={skills.length}
                inputRef={search}
              />
            ) : null}
            <SkillsList
              skills={skills}
              usedBy={usedBy}
              query={query}
              onCreate={() => void startCreate()}
              onClear={() => {
                setQuery("")
                search.current?.focus()
              }}
              selected={id}
              onSelect={(skill) => void navigate({ search: { skill } })}
              actions={(name) => actions(name)}
            />
          </div>
        )}
      </div>
      {id || creating ? (
        <SkillInspector
          id={current!}
          isNew={creating}
          usedBy={id && !creating ? usedBy.of(id) : null}
          project={usedBy.project}
          doc={creating ? { name: "", description: "", body: "" } : doc}
          error={error}
          form={form}
          footer={doc && !creating ? actions(doc.name) : undefined}
          dirty={dirty}
          saving={saving}
          saveError={editing ? undefined : saveError}
          editing={!!editing}
          onEdit={() =>
            (doc || creating) &&
            !saving &&
            setEditor({ id: current!, body: draft.body })
          }
          onClose={() => {
            if (saving) return
            if (creating) {
              void (async () => {
                if (!dirty || (await discard.ask("Discard this new Skill?")))
                  setCreating(false)
              })()
              return
            }
            void navigate({ search: {} })
          }}
        />
      ) : null}
      {discard.dialog}
    </div>
  )
}

export function SkillsError({
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
          Couldn’t load skills:{" "}
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
