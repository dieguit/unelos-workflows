// Run with Playwright MCP browser_run_code_unsafe against the dev mock server.
async (page) => {
  await page.setViewportSize({ width: 1440, height: 900 })
  await page.goto("http://localhost:3000/workflows")
  await page.waitForSelector("tbody tr[data-row]")
  await page.evaluate(() => {
    const bridge = window.__TAURI_INTERNALS__
    const invoke = bridge.invoke
    let doc = {
      id: "implement-feature",
      name: "implement-feature",
      description: null,
      start: "plan",
      steps: [{ id: "plan", originalId: "plan", task: { source: "inline", id: null, description: null, instructions: "Plan", skills: [], outcomes: [] }, label: "plan", kind: "task", title: "write-plan", agent: "planner", execution: "main", routes: [], workers: [], inputs: [], outputs: [] }],
      issues: [],
    }
    let fail = true
    bridge.invoke = async (cmd, args, ...rest) => {
      if (cmd === "get_workflow") return structuredClone(doc)
      if (cmd === "validate_workflow") {
        await new Promise((resolve) => setTimeout(resolve, 200))
        return args.draft.name === "invalid" ? [{ message: "Invalid workflow", step: null }] : []
      }
      if (cmd === "save_workflow") {
        if (fail) {
          fail = false
          throw "Save failed"
        }
        await new Promise((resolve) => setTimeout(resolve, 700))
        doc = { ...doc, ...args.draft, id: args.draft.name }
        return structuredClone(doc)
      }
      return invoke(cmd, args, ...rest)
    }
  })
  await page.locator("tbody tr").first().click()
  const name = page.getByLabel("Name", { exact: true })
  const save = page.getByRole("button", { name: "Save", exact: true })
  await name.waitFor()
  await name.fill("invalid")
  await page.getByText("1 issue").waitFor()
  await save.click()
  if ((await name.inputValue()) !== "invalid") throw new Error("Invalid draft changed")
  await name.fill("renamed")
  await page.getByRole("button", { name: "Close settings" }).click()
  await page.getByRole("button", { name: "Workflow settings" }).click()
  if ((await name.inputValue()) !== "renamed")
    throw new Error("Closing lost draft")
  await page.getByText("Valid", { exact: true }).waitFor()
  await save.click()
  await page.getByText(/Could not save workflow/).waitFor()
  if ((await name.inputValue()) !== "renamed")
    throw new Error("Failed save lost draft")
  await save.click()
  await page.getByRole("button", { name: "Saving…" }).waitFor()
  const projectMenu = page.locator('button[aria-haspopup="menu"]').first()
  await projectMenu.click()
  await page.getByRole("menuitem", { name: "api-gateway", exact: true }).click()
  if (!(await projectMenu.textContent()).includes("unelos-app"))
    throw new Error("Project switched during save")
  await page.getByText("renamed.yaml", { exact: true }).waitFor()
  await name.fill("unsaved")
  await projectMenu.click()
  await page.getByRole("menuitem", { name: "api-gateway", exact: true }).click()
  await page.getByRole("button", { name: "Stay" }).click()
  if (
    !(await projectMenu.textContent()).includes("unelos-app") ||
    (await name.inputValue()) !== "unsaved"
  )
    throw new Error("Stay changed project or lost draft")
  await page.getByRole("button", { name: "Workflows", exact: true }).click()
  await page.getByRole("button", { name: "Stay" }).click()
  if ((await name.inputValue()) !== "unsaved")
    throw new Error("Stay lost draft")
  await projectMenu.click()
  await page.getByRole("menuitem", { name: "api-gateway", exact: true }).click()
  await page.getByRole("button", { name: "Discard changes" }).click()
  await page.waitForFunction(() =>
    document
      .querySelector('button[aria-haspopup="menu"]')
      ?.textContent.includes("api-gateway")
  )
  return "Workflow settings check passed"
}
