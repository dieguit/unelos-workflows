import { useDelayedFlag } from "@/hooks/use-delayed-flag"
import { useEffect, useRef, useState } from "react"
import { createFileRoute, useBlocker, useRouter } from "@tanstack/react-router"
import { toast } from "sonner"
import { z } from "zod"
import { useStore } from "@tanstack/react-form"
import { Button } from "@/components/ui/button"
import { ListSearch } from "@/components/list-search"
import { BodyEditor } from "@/components/body-editor"
import { PageHeader } from "@/components/page-header"
import { ListActions } from "@/components/list-actions"
import {
  listSkills,
  duplicateSkill,
  removeSkill,
  requireProject,
  saveSkill,
  type SkillSaveResult,
} from "@/lib/projects"
import { SkillInspector } from "./-components/skill-inspector"
import { useSkillForm } from "./-hooks/use-skill-form"
import { useDiscardChanges } from "@/hooks/use-discard-changes"
import { useProjectDiscardGuard } from "@/hooks/use-project-discard-guard"
import { applyBodyDraft } from "@/lib/apply-body-draft"
import { SkillsList, filterSkills } from "./-components/skills-list"
import { useSkill } from "./-hooks/use-skill"
import { skillSchema } from "./-lib/skill-schema"

export const Route = createFileRoute("/skills/")({
  validateSearch: z.object({ skill: z.string().optional().catch(undefined) }),
  beforeLoad: requireProject,
  loader: async ({ context }) => ({
    root: context.project.path,
    skills: await listSkills(context.project.path),
  }),
  component: SkillsRoute,
  pendingComponent: () => <p role="status">Loading skills…</p>,
  errorComponent: SkillsError,
})

function Header() {
  return (
    <PageHeader
      title="Skills"
      description="Skill definitions for this project."
    />
  )
}

function SkillsRoute() {
  const { project } = Route.useRouteContext()
  return <SkillsHome key={project.path} />
}

function SkillsHome() {
  const { root, skills } = Route.useLoaderData()
  const { project } = Route.useRouteContext()
  const [query, setQuery] = useState("")
  const search = useRef<HTMLInputElement>(null)
  const { skill: id } = Route.useSearch()
  const navigate = Route.useNavigate()
  const router = useRouter()
  const { doc, error, setDoc } = useSkill(root, id)
  const [saveError, setSaveError] = useState<string>()
  const [editor, setEditor] = useState<{ id: string; body: string } | null>(
    null
  )
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
  const form = useSkillForm(
    doc
      ? { ...doc, description: doc.description ?? "" }
      : { name: "", description: "", body: "" },
    async (value, closeEditor) => {
      if (!doc) return
      setSaveError(undefined)
      try {
        const res = await saveSkill(root, id!, value)
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
  const [previousDoc, setPreviousDoc] = useState(doc)
  if (previousDoc !== doc) {
    setPreviousDoc(doc)
    setSaveError(undefined)
  }
  useEffect(() => {
    form.reset(
      doc
        ? { ...doc, description: doc.description ?? "" }
        : { name: "", description: "", body: "" }
    )
  }, [doc, form])
  const dirty = !!doc && !isDefaultValue
  const [previousId, setPreviousId] = useState(id)
  if (previousId !== id) {
    setPreviousId(id)
    setEditor(null)
  }
  const editing = editor && doc && editor.id === id ? editor : null

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

  if (root !== project.path)
    return (
      <div className="flex flex-col gap-6">
        <Header />
        <p role="status">Loading skills…</p>
      </div>
    )

  return (
    // Cancel <main>'s padding so the inspector sits flush right at full height.
    <div inert={saving} className="-mx-8 -mt-6 -mb-8 flex h-[calc(100%+3.5rem)]">
      <div className="min-w-0 flex-1 overflow-y-auto px-8 pt-6 pb-8">
        {editing ? (
          <div className="flex h-full flex-col gap-4">
            <BodyEditor
              title={editing.id}
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
            <Header />
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
              query={query}
              onClear={() => {
                setQuery("")
                search.current?.focus()
              }}
              selected={id}
              onSelect={(skill) => void navigate({ search: { skill } })}
              actions={(name) => (
                <ListActions
                  kind="Skill"
                  name={name}
                  nameSchema={skillSchema.shape.name}
                  disabled={visualSaving}
                  beforeRemove={async () =>
                    id !== name ||
                    (!saving &&
                      (!dirty ||
                        (await discard.ask(
                          "Discard unsaved changes to this Skill?"
                        ))))
                  }
                  duplicate={async (newId) => {
                    await duplicateSkill(root, name, newId)
                    void router.invalidate()
                  }}
                  remove={async () => {
                    await removeSkill(root, name)
                    if (id === name)
                      await navigate({ search: {}, ignoreBlocker: true })
                    void router.invalidate()
                  }}
                />
              )}
            />
          </div>
        )}
      </div>
      {id ? (
        <SkillInspector
          id={id}
          doc={doc}
          error={error}
          form={form}
          dirty={dirty}
          saving={saving}
          saveError={editing ? undefined : saveError}
          editing={!!editing}
          onEdit={() => doc && !saving && setEditor({ id, body: draft.body })}
          onClose={() => {
            if (saving) return
            void navigate({ search: {} })
          }}
        />
      ) : null}
      {discard.dialog}
    </div>
  )
}

function SkillsError({ error }: { error: unknown }) {
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
