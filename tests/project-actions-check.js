// Run with Playwright MCP browser_run_code_unsafe(filename: this path).
async (page) => {
  const assert = (ok, message) => { if (!ok) throw new Error(message) }
  await page.setViewportSize({ width: 1440, height: 900 })
  for (const pick of ["cancel", "plain"]) {
    await page.goto(`http://localhost:3000/?mock=none&pick=${pick}`)
    await page.getByRole("button", { name: "Open project folder" }).click()
    if (pick === "plain") await page.getByText("plain is not a Unelos project (missing .unelos/project.yaml)").waitFor()
    assert(await page.getByRole("heading", { name: "Open a project to get started" }).isVisible(), `${pick} must not open a project`)
    assert(await page.evaluate(async () => (await window.__TAURI_INTERNALS__.invoke("project_state")).recents.length === 0), `${pick} must preserve state`)
  }
  await page.goto("http://localhost:3000/?mock=none")
  await page.getByRole("button", { name: "Open project folder" }).click()
  await page.waitForSelector("tbody tr[data-row]")
  assert(await page.locator('button[aria-haspopup="menu"]').first().innerText().then((s) => s.includes("api-gateway")), "Native picker command result must update the current project")
  const blocked = await page.evaluate(async () => {
    const results = []
    for (const [command, args] of [["open_project", { path: "/unapproved" }], ["list_workflows", { root: "/unapproved" }]]) {
      try { await window.__TAURI_INTERNALS__.invoke(command, args); results.push(false) }
      catch { results.push(true) }
    }
    return results.every(Boolean)
  })
  assert(blocked, "Dev IPC must mirror backend rejection of arbitrary roots")
  await page.evaluate(() => Object.defineProperty(navigator, "clipboard", { configurable: true, value: undefined }))
  await page.locator('button[aria-haspopup="menu"]').first().click()
  await page.getByRole("menuitem", { name: "Copy path" }).click()
  await page.locator('[data-sonner-toast][data-type="error"]').waitFor()
  await page.locator('button[aria-haspopup="menu"]').first().click()
  await page.getByRole("menuitem", { name: "Reveal in file manager" }).click()
  await page.goto("http://localhost:3000/?mock=unavailable")
  const alert = await page.getByRole("alert").innerText()
  assert(alert.includes("/home/dieguit/projects/unelos/unelos-app") && alert.includes(".unelos/project.yaml"), "Unavailable alert must show full path and account for missing config")
  await page.goto("http://localhost:3000/workflows?mock=empty")
  await page.getByText("Create your first workflow").waitFor()
  await page.goto("http://localhost:3000/workflows?mock=error")
  await page.getByText(/Couldn’t load workflows:/).waitFor()
  assert(await page.getByRole("button", { name: "Retry" }).isVisible(), "Error state needs Retry")
  return "Project actions check passed (pick/open/cancel/reject, authorization mock, missing clipboard toast, reveal, unavailable/empty/error)"
}
