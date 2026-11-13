import * as mod from "@/routes/harnesses/-lib/harnesses-list"
import { expect, test } from "vitest"
import type { Harness } from "@/lib/harnesses"


const h = (id: string, name: string, o: Partial<Harness> = {}): Harness => ({
  id,
  name,
  installable: false,
  detected: false,
  installed: false,
  updateAvailable: false,
  version: null,
  integrationVersion: null,
  binary: null,
  configDir: null,
  ...o,
})

const harnesses = [
  h("zeta", "Zeta", { installable: true }),
  h("claude", "Claude Code", { detected: true, version: "2.3.0" }),
  h("amp", "Amp"),
  h("beta", "Beta", { installable: true, detected: true }),
  h("opencode", "OpenCode", {
    installable: true,
    detected: true,
    installed: true,
  }),
  h("alpha", "Alpha", { installable: true }),
]

test("sortHarnesses: installed → detected → not detected → non-installable, alpha within", () => {
  expect(mod.sortHarnesses(harnesses).map((x) => x.id)).toEqual(["opencode", "beta", "alpha", "zeta", "claude", "amp"])
})

test("filterHarnesses matches name or id, case-insensitive, trimmed, keeps order", () => {
  const ids = (q: string) => mod.filterHarnesses(harnesses, q).map((x) => x.id)
  expect(ids(" CLAUDE ")).toEqual(["claude"])
  expect(ids("code")).toEqual(["opencode", "claude"])
  expect(ids("a").slice(0, 2)).toEqual(["beta", "alpha"])
  expect(ids("nope")).toEqual([])
  expect(ids("").length).toBe(harnesses.length)
})
