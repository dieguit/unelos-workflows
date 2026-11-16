import { beforeEach, expect, test, vi } from "vitest"
import { toast } from "sonner"
import { trackedInvoke } from "./tauri"
import { autoBuildHarness, buildHarnessConfig, uninstallProject } from "./projects"

vi.mock("./tauri", () => ({ trackedInvoke: vi.fn() }))
vi.mock("sonner", () => ({ toast: { warning: vi.fn(), error: vi.fn() } }))

beforeEach(() => vi.resetAllMocks())

test("uninstall invokes only its command and passes through state and errors", async () => {
  const state = { home: "/home", recents: [] }
  vi.mocked(trackedInvoke).mockResolvedValueOnce(state)
  expect(await uninstallProject("/project")).toBe(state)
  expect(trackedInvoke).toHaveBeenCalledExactlyOnceWith("uninstall_project", { root: "/project" })
  vi.mocked(trackedInvoke).mockRejectedValueOnce(new Error("refused"))
  await expect(uninstallProject("/project")).rejects.toThrow("refused")
  expect(trackedInvoke).toHaveBeenCalledTimes(2)
})

test("manual and automatic builds warn once with skipped entries and mapping guidance", async () => {
  const result = { harnesses: ["opencode"], written: [], removed: [], skipped: ["claude: No Claude model mapped for alias 'smart'", "invalid workflow"] }
  vi.mocked(trackedInvoke).mockResolvedValue(result)
  for (const build of [autoBuildHarness, buildHarnessConfig]) {
    vi.mocked(toast.warning).mockClear()
    expect(await build("/project")).toBe(result)
    expect(toast.warning).toHaveBeenCalledExactlyOnceWith(
      expect.stringContaining(`${result.skipped.join("; ")}. Check Model mappings`)
    )
  }
})

test("disabled auto-build and complete builds do not warn", async () => {
  vi.mocked(trackedInvoke).mockResolvedValueOnce(null).mockResolvedValueOnce({ harnesses: [], written: [], removed: [], skipped: [] })
  await autoBuildHarness("/project")
  await buildHarnessConfig("/project")
  expect(toast.warning).not.toHaveBeenCalled()
})
