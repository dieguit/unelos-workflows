import { beforeEach, expect, test, vi } from "vitest"
import { createElement, type ReactNode } from "react"
import { renderToStaticMarkup } from "react-dom/server"
import { uninstallProject } from "@/lib/projects"
import { requestProjectChange } from "@/lib/project-change-guard"
import { toast } from "sonner"
import { UninstallProject } from "./uninstall-project"

const h = vi.hoisted(() => ({
  states: [] as unknown[], refs: [] as { current: unknown }[], state: 0, ref: 0,
  buttons: [] as { children?: ReactNode; disabled?: boolean; onClick?: () => Promise<void> }[],
  navigate: vi.fn(), invalidate: vi.fn(), release: vi.fn(),
  changeOpen: undefined as undefined | ((open: boolean) => void),
}))
vi.mock("react", async (original) => ({
  ...await original<typeof import("react")>(),
  useState: (initial: unknown) => {
    const i = h.state++
    if (!(i in h.states)) h.states[i] = initial
    return [h.states[i], (value: unknown) => { h.states[i] = value }]
  },
  useRef: (initial: unknown) => {
    const i = h.ref++
    return h.refs[i] ??= { current: initial }
  },
}))
vi.mock("@tanstack/react-router", () => ({ useRouter: () => ({ navigate: h.navigate, invalidate: h.invalidate }) }))
vi.mock("@/lib/projects", () => ({ uninstallProject: vi.fn() }))
vi.mock("@/lib/project-change-guard", () => ({ requestProjectChange: vi.fn() }))
vi.mock("sonner", () => ({ toast: { error: vi.fn() } }))
vi.mock("@/components/ui/button", () => ({
  Button: (props: typeof h.buttons[number]) => { h.buttons.push(props); return createElement("button", { disabled: props.disabled }, props.children) },
}))
vi.mock("@/components/ui/dialog", () => {
  const part = ({ children }: { children: ReactNode }) => createElement("div", null, children)
  return {
    Dialog: ({ children, onOpenChange }: { children: ReactNode; onOpenChange: typeof h.changeOpen }) => {
      h.changeOpen = onOpenChange
      return part({ children })
    },
    DialogContent: part, DialogDescription: part, DialogFooter: part, DialogTitle: part,
    DialogTrigger: part,
    DialogClose: ({ children }: { children: ReactNode }) => createElement("button", { onClick: () => h.changeOpen?.(false) }, children),
  }
})

const root = "/temporary/project"
function render() {
  h.state = 0; h.ref = 0; h.buttons = []
  return renderToStaticMarkup(createElement(UninstallProject, { root }))
}
function confirm() {
  render()
  return h.buttons.find((button) => button.children === "Uninstall")!.onClick!()
}
beforeEach(() => {
  vi.resetAllMocks()
  h.states = []; h.refs = []
  vi.mocked(requestProjectChange).mockResolvedValue(h.release)
  vi.mocked(uninstallProject).mockResolvedValue({ home: "/home", recents: [{ path: "/other", name: "other", openedAt: 1, available: true }] })
})

test("confirmation identifies project and deletion scope; cancelling does not mutate", () => {
  const html = render()
  expect(html).toContain(root)
  expect(html).toContain("Delete .unelos (including workflows and run history) and generated Unelos agents? Your other project files and global integrations will remain.")
  expect(html).toContain("Cancel")
  h.changeOpen!(false)
  expect(uninstallProject).not.toHaveBeenCalled()
})

test("confirmed uninstall navigates to explicit picker before invalidating deleted loaders", async () => {
  await confirm()
  expect(uninstallProject).toHaveBeenCalledExactlyOnceWith(root)
  expect(h.navigate).toHaveBeenCalledExactlyOnceWith({ to: "/", search: { picker: true } })
  expect(h.navigate.mock.invocationCallOrder[0]).toBeLessThan(h.invalidate.mock.invocationCallOrder[0]!)
  expect(h.release).toHaveBeenCalledOnce()
})

test("edit guard cancellation does not uninstall or navigate", async () => {
  vi.mocked(requestProjectChange).mockResolvedValue(null)
  await confirm()
  expect(uninstallProject).not.toHaveBeenCalled()
  expect(h.navigate).not.toHaveBeenCalled()
})

test("IPC error is shown without navigation and releases guard", async () => {
  vi.mocked(uninstallProject).mockRejectedValue(new Error("unsafe path"))
  await confirm()
  expect(toast.error).toHaveBeenCalledWith(expect.stringContaining("unsafe path"))
  expect(h.navigate).not.toHaveBeenCalled()
  expect(h.invalidate).not.toHaveBeenCalled()
  expect(h.release).toHaveBeenCalledOnce()
})

test("pending guard and backend prevent duplicate submission and disable action", async () => {
  let releaseGuard!: (release: () => void) => void
  vi.mocked(requestProjectChange).mockReturnValue(new Promise((resolve) => { releaseGuard = resolve }))
  let finish!: () => void
  vi.mocked(uninstallProject).mockReturnValue(new Promise((resolve) => { finish = () => resolve({ home: "", recents: [] }) }))
  const first = confirm()
  render()
  expect(h.buttons.filter((b) => b.disabled).length).toBeGreaterThan(0)
  h.changeOpen!(false)
  // Invoke the original event again, as a rapid double click can precede React's render.
  const action = h.buttons.find((b) => b.children === "Uninstalling…")!
  await action.onClick!()
  expect(requestProjectChange).toHaveBeenCalledOnce()
  releaseGuard(h.release)
  await Promise.resolve()
  await action.onClick!()
  expect(uninstallProject).toHaveBeenCalledExactlyOnceWith(root)
  finish()
  await first
  expect(h.release).toHaveBeenCalledOnce()
  render()
  expect(h.buttons.find((b) => b.children === "Uninstall")?.disabled).toBe(false)
})
