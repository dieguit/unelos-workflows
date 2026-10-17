// Run with Playwright MCP browser_run_code_unsafe(filename: this path).
async (page) => {
  const assert = (ok, message) => { if (!ok) throw new Error(message) }
  await page.setViewportSize({ width: 1440, height: 900 })
  await page.goto("http://localhost:3000/workflows?mock=switch")
  await page.waitForSelector("tbody tr[data-row]")
  await page.evaluate(() => {
    window.projectSamples = []
    const sample = () => {
      if (!location.pathname.startsWith("/workflows")) return
      const project = document.querySelector('button[aria-haspopup="menu"] .font-semibold')?.textContent
      const names = [...document.querySelectorAll("tbody tr[data-row] td:first-child > span:first-child")].map((el) => el.textContent)
      window.projectSamples.push({ project, names })
    }
    new MutationObserver(sample).observe(document.body, { subtree: true, childList: true, characterData: true })
    sample()
  })
  const switchTo = async (name) => {
    await page.locator('button[aria-haspopup="menu"]').first().click()
    await page.getByRole("menuitem", { name, exact: true }).click()
    await page.waitForFunction((name) => document.querySelector("tbody tr[data-row]")?.textContent.includes(`${name}:`), name)
  }
  await switchTo("api-gateway")
  // Workflows is now cached. Switching from another section must not revive it.
  await page.getByRole("link", { name: "Agents", exact: true }).click()
  await page.waitForURL("**/agents")
  await switchTo("docs-site")
  // Also revisit cached Workflows after project state changes on another route.
  await page.getByRole("link", { name: "Agents", exact: true }).click()
  await page.waitForURL("**/agents")
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke("open_project", { path: "/home/dieguit/projects/unelos/unelos-app" })
    window.dispatchEvent(new Event("focus"))
  })
  await page.waitForFunction(() => document.querySelector('button[aria-haspopup="menu"]')?.textContent.includes("unelos-app"))
  await page.getByRole("link", { name: "Workflows", exact: true }).click()
  await page.waitForFunction(() => document.querySelector("tbody tr[data-row]")?.textContent.includes("unelos-app:"))
  const samples = await page.evaluate(() => window.projectSamples)
  const mismatches = samples.filter(({ project, names }) => names.some((name) => !name.startsWith(`${project}:`)))
  assert(!mismatches.length, `Cross-project rows exposed: ${JSON.stringify(mismatches)}`)
  assert(new Set(samples.filter((s) => s.names.length).map((s) => s.project)).size === 3, "Must observe distinct rows for all three projects")
  return `Project switch check passed (${samples.length} DOM samples, direct switch + cached navigation from Agents)`
}
