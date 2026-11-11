export function applyBodyDraft(body: string, update: (body: string) => void, close: () => void) {
  update(body)
  close()
}
