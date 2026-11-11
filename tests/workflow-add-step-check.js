// Run with Playwright MCP browser_run_code_unsafe against the dev mock server.
async (page) => {
  await page.setViewportSize({ width: 1440, height: 900 })
  await page.goto("http://localhost:3000/workflows")
  await page.waitForSelector("tbody tr[data-row]")
  await page.evaluate(() => {
    const bridge = window.__TAURI_INTERNALS__
    const invoke = bridge.invoke
    window.__saved = null
    bridge.invoke = async (cmd, args, ...rest) => {
      if (cmd === "save_workflow") window.__saved = structuredClone(args.draft)
      return invoke(cmd, args, ...rest)
    }
  })
  await page.getByRole("row", { name: /implement-feature/ }).click()
  const pill = page.getByRole("button", { name: "Add step" }).first()
  await pill.click()
  await page.getByRole("menuitem", { name: "Human gate" }).waitFor()
  await page.keyboard.press("Escape")
  await page.getByRole("menuitem", { name: "Task" }).waitFor({ state: "detached" })
  await pill.click()
  await page.getByRole("menuitem", { name: "Task" }).click()
  const name = page.getByLabel("Name", { exact: true })
  await name.waitFor()
  if (!(await name.evaluate((el) => el === document.activeElement)))
    throw new Error("Name not focused after insert")
  if (await page.getByText(/^\d+ issues?$/).count())
    throw new Error("Pristine new step shows issues")
  if (await page.getByText(/missing (required )?field/).count())
    throw new Error("Raw core issue shown")
  await page.getByRole("button", { name: "Validate" }).click()
  await page.getByText("Choose an agent.").first().waitFor()
  await page.getByText("Choose where it runs.").first().waitFor()
  const save = page.getByRole("button", { name: "Save", exact: true })
  if (await save.isEnabled()) throw new Error("Save enabled for incomplete task")
  await name.fill("Verify changes")
  await page.getByText("verify-changes").first().waitFor()
  await page.getByLabel("Agent", { exact: true }).click()
  await page.getByRole("option").first().click()
  await page.getByRole("button", { name: "Subagent", exact: true }).click()
  await page.waitForFunction(() => [...document.querySelectorAll("button")].some((b) => b.textContent.trim() === "Save" && !b.disabled))
  await save.click()
  await page.waitForFunction(() => window.__saved)
  const draft = await page.evaluate(() => window.__saved)
  const ids = draft.steps.map((s) => s.id)
  const i = ids.indexOf("verify-changes")
  if (i !== 1) throw new Error(`Inserted at ${i}: ${ids}`)
  const prev = draft.steps[0].routes.map((r) => r.target)
  if (!prev.includes("verify-changes")) throw new Error(`Source not rerouted: ${prev}`)
  const step = draft.steps[i]
  if (step.routes[0]?.target !== ids[2] || step.execution !== "subagent" || !step.agent || step.originalId !== null)
    throw new Error(`Bad new step ${JSON.stringify(step)}`)
  return "Workflow add step check passed"
}
