let guard: (() => Promise<(() => void) | null>) | undefined

export function registerProjectChangeGuard(
  next: () => Promise<(() => void) | null>
) {
  guard = next
  return () => {
    if (guard === next) guard = undefined
  }
}

export function requestProjectChange() {
  return guard ? guard() : Promise.resolve(() => {})
}
