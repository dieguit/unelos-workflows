// Run with Playwright MCP browser_run_code_unsafe against the dev mock server.
async (page) => {
  const cases = [
    {
      path: "agents",
      name: "engineer",
      existing: "engineer",
      title: "Agent engineer",
    },
    {
      path: "skills",
      name: "code-review",
      existing: "code-review",
      title: "Skill code-review",
    },
    {
      path: "workflows",
      name: "Implement feature",
      existing: "implement-feature",
      title: "Workflow settings",
    },
  ]
  for (const item of cases) {
    await page.goto(`http://localhost:3000/${item.path}`)
    await page.evaluate(() => {
      const bridge = window.__TAURI_INTERNALS__
      const invoke = bridge.invoke
      bridge.invoke = async (cmd, ...args) => {
        if (cmd.startsWith("duplicate_"))
          await new Promise((resolve) => setTimeout(resolve, 300))
        return invoke(cmd, ...args)
      }
    })
    const row = page
      .locator("tbody tr[data-row]")
      .filter({ hasText: item.name })
      .first()
    await row.waitFor()
    const trigger = row.getByRole("button", {
      name: `Actions for ${item.name}`,
    })
    await trigger.focus()
    await page.keyboard.press("Enter")
    await page.getByRole("menuitem", { name: "Duplicate" }).waitFor()
    if (await page.getByRole("complementary", { name: item.title }).count())
      throw new Error(`${item.path}: menu opened row`)
    await page.getByRole("menuitem", { name: "Duplicate" }).click()
    await page
      .getByRole("dialog")
      .getByLabel("Name", { exact: true })
      .fill(item.existing)
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Duplicate", exact: true })
      .click()
    if (
      await page
        .getByRole("dialog")
        .getByLabel("Name", { exact: true })
        .isEnabled()
    )
      throw new Error(`${item.path}: name editable while duplicating`)
    await page.getByRole("dialog").getByRole("alert").waitFor()
    const copy = `ui-copy-${Date.now()}`
    await page
      .getByRole("dialog")
      .getByLabel("Name", { exact: true })
      .fill(copy)
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Duplicate", exact: true })
      .click()
    const copyRow = page
      .locator("tbody tr[data-row]")
      .filter({ hasText: copy })
      .first()
    await copyRow.waitFor()
    await copyRow.getByRole("button", { name: `Actions for ${copy}` }).click()
    await page.getByRole("menuitem", { name: "Remove" }).click()
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Remove", exact: true })
      .click()
    await copyRow.waitFor({ state: "detached" })
    await row.click()
    await page.getByRole("complementary", { name: item.title }).waitFor()
  }
  return "List actions and row opening passed"
}
