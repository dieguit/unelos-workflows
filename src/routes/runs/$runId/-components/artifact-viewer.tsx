import { useEffect, useState } from "react"
import { BodyEditor } from "@/components/body-editor"
import { Button } from "@/components/ui/button"
import { readRunArtifact, type ArtifactReference } from "../../-lib/runs-api"

export function ArtifactViewer({
  root,
  runId,
  artifact,
  onClose,
}: {
  root: string
  runId: string
  artifact: ArtifactReference
  onClose: () => void
}) {
  const [result, setResult] = useState<{ path: string; body?: string; error?: string }>()
  useEffect(() => {
    let active = true
    readRunArtifact(root, runId, artifact.path).then(
      (body) => active && setResult({ path: artifact.path, body }),
      (err) => active && setResult({ path: artifact.path, error: String(err) })
    )
    return () => {
      active = false
    }
  }, [root, runId, artifact.path])
  const current = result?.path === artifact.path ? result : undefined
  if (current?.body === undefined)
    return (
      <div className="flex min-w-0 flex-1 flex-col gap-4 p-4">
        <header className="flex items-center gap-3">
          <h1 className="min-w-0 flex-1 truncate text-xl font-semibold">{artifact.name}</h1>
          <Button variant="secondary" onClick={onClose}>
            Close
          </Button>
        </header>
        {current?.error ? (
          <p role="alert" className="text-sm [overflow-wrap:anywhere] text-destructive">
            Could not read {artifact.path}: {current.error}
          </p>
        ) : (
          <p role="status">Loading…</p>
        )}
      </div>
    )
  return (
    <div className="flex min-w-0 flex-1 flex-col p-4">
      <BodyEditor readOnly title={artifact.name} label={artifact.path} body={current.body} onCancel={onClose} />
    </div>
  )
}
