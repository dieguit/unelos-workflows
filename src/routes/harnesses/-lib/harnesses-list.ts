import type { Harness } from "@/lib/harnesses"

const rank = (h: Harness) =>
  (h.installed ? 0 : !h.installable ? 3 : h.detected ? 1 : 2) * 2 +
  (h.detected ? 0 : 1)

/** installed → detected installable → not detected installable → non-installable (detected first); alpha within. */
export function sortHarnesses(harnesses: Harness[]) {
  return [...harnesses].sort(
    (a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name)
  )
}

export function filterHarnesses(harnesses: Harness[], query: string) {
  const needle = query.trim().toLowerCase()
  return sortHarnesses(harnesses).filter((h) =>
    [h.name, h.id].some((text) => text.toLowerCase().includes(needle))
  )
}
