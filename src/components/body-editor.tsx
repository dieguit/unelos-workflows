import { useState } from "react"
import { MarkdownEditor } from "@/components/markdown-editor"
import { Button } from "@/components/ui/button"
import { Field, FieldLabel } from "@/components/ui/field"

/** Full-height Markdown editor that opens over the main content (SKILL.md, Agent instructions). */
export function BodyEditor({
  title,
  label,
  body,
  onChange,
  onCancel,
  onSave,
  onBlur,
}: {
  title: string
  label: string
  body: string
  onChange: (body: string) => void
  onCancel: () => void
  onSave: () => Promise<void>
  onBlur?: () => void
}) {
  const [saving, setSaving] = useState(false)
  return (
    <div className="flex h-full min-h-96 flex-col gap-4">
      <header className="flex items-center gap-3">
        <h1 className="flex min-w-0 flex-1 items-baseline gap-2 text-xl font-semibold">
          <span className="truncate">{title}</span>
          <span className="font-mono text-sm font-normal text-muted-foreground">
            {label}
          </span>
        </h1>
        <Button variant="secondary" onClick={onCancel} disabled={saving}>
          Cancel
        </Button>
        <Button
          disabled={saving}
          onClick={() => {
            setSaving(true)
            void onSave().finally(() => setSaving(false))
          }}
        >
          {saving ? "Applying…" : "Apply"}
        </Button>
      </header>
      <Field className="min-h-0 flex-1" onBlur={onBlur}>
        <FieldLabel className="sr-only">
          {title} {label}
        </FieldLabel>
        <MarkdownEditor
          autoFocus
          aria-label={`${title} ${label}`}
          value={body}
          onChange={onChange}
          className="flex-1"
        />
      </Field>
    </div>
  )
}
