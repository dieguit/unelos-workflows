import { expect, test, vi } from "vitest"

vi.mock("./no-project", () => ({ NoProject: () => null }))
vi.mock("@tanstack/react-router", () => ({
  lazyRouteComponent: () => () => null,
  createFileRoute: () => (options: unknown) => ({ options }),
  redirect: (options: unknown) => options,
}))

import { Route } from "../index"

const options = Route.options as unknown as {
  validateSearch: (search: Record<string, unknown>) => { picker?: boolean }
  beforeLoad: (args: { context: { project: object | null }; search: { picker?: boolean } }) => void
}

test("ordinary startup redirects but explicit picker preserves other recents", () => {
  expect(() => options.beforeLoad({ context: { project: { path: "/other" } }, search: {} })).toThrow()
  expect(() => options.beforeLoad({ context: { project: { path: "/other" } }, search: { picker: true } })).not.toThrow()
  expect(() => options.beforeLoad({ context: { project: null }, search: {} })).not.toThrow()
})

test("picker accepts only an explicit true boolean", () => {
  expect(options.validateSearch({ picker: true })).toEqual({ picker: true })
  expect(options.validateSearch({ picker: "false" })).toEqual({ picker: undefined })
})
