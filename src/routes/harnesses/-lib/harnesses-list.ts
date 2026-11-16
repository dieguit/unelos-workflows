import { sortHarnesses, type Harness } from "@/lib/harnesses"
export { sortHarnesses } from "@/lib/harnesses"

export function filterHarnesses(harnesses: Harness[], query: string) {
  const needle = query.trim().toLowerCase()
  return sortHarnesses(harnesses).filter((h) =>
    [h.name, h.id].some((text) => text.toLowerCase().includes(needle))
  )
}
