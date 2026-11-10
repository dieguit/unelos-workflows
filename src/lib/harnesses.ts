import { trackedInvoke } from "./tauri"

export type Harness = {
  id: string
  name: string
  installable: boolean
  detected: boolean
  installed: boolean
  updateAvailable: boolean
  version: string | null
  integrationVersion: string | null
  binary: string | null
  configDir: string | null
}

export const listHarnesses = () => trackedInvoke<Harness[]>("list_harnesses")
export const installHarness = (id: string) =>
  trackedInvoke<Harness[]>("install_harness", { id })
export const uninstallHarness = (id: string) =>
  trackedInvoke<Harness[]>("uninstall_harness", { id })
