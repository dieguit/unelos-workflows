// Run with Playwright MCP browser_run_code_unsafe against the stateful dev mock.
async (page) => {
  await page.goto("http://localhost:3000/workflows")
  await page
    .locator("tbody tr[data-row]")
    .filter({ hasText: "Investigate bug" })
    .click()
  const description = page.getByLabel("Description", { exact: true })
  const name = page.getByLabel("Name", { exact: true })
  const save = page.getByRole("button", { name: "Save", exact: true })
  await description.fill("Verified workflow settings")
  await page.waitForFunction(() => [...document.querySelectorAll("button")].some((b) => b.textContent.trim() === "Save" && !b.disabled))
  await save.click()
  await page.getByText("Unsaved changes").waitFor({ state: "hidden" })
  if ((await description.inputValue()) !== "Verified workflow settings")
    throw new Error("First save reverted the form")
  await name.fill("verified-investigation")
  await page.waitForFunction(() => [...document.querySelectorAll("button")].some((b) => b.textContent.trim() === "Save" && !b.disabled))
  await save.click()
  await page.getByText("verified-investigation.yaml", { exact: true }).waitFor()
  await page.getByText("Unsaved changes").waitFor({ state: "hidden" })
  await page.getByRole("button", { name: "Workflows", exact: true }).click()
  await page
    .locator("tbody tr[data-row]")
    .filter({ hasText: "verified-investigation" })
    .click()
  if ((await description.inputValue()) !== "Verified workflow settings")
    throw new Error("Rename overwrote the previously saved description")
  const stored = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("get_workflow", {
      root: "/home/dieguit/projects/unelos/unelos-app",
      id: "verified-investigation",
    })
  )
  if (stored.description !== "Verified workflow settings")
    throw new Error("Saved document has the wrong description")
  return "Sequential workflow settings saves passed"
}
