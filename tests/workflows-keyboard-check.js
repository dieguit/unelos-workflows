// Run with Playwright MCP browser_run_code_unsafe(filename: this path).
async (page) => {
  const assert = (ok, message) => { if (!ok) throw new Error(message) }
  await page.setViewportSize({ width: 1440, height: 900 })
  await page.goto("http://localhost:3000/workflows")
  const rows = page.locator("tbody tr[data-row]")
  await rows.first().waitFor()
  await rows.first().focus()
  await page.keyboard.press("Tab")
  const description = "Plan, approve, implement and review a feature with a correction loop."
  assert(await page.evaluate((text) => document.activeElement?.textContent === text, description), "Tab must focus full description")
  await page.locator('[data-slot="tooltip-content"]').filter({ hasText: description }).waitFor()
  await page.keyboard.press("Escape")
  await rows.nth(2).focus()
  await page.keyboard.press("Tab")
  const diagnostic = "nodes.review.agent: Agent reference missing"
  assert(await page.evaluate((text) => document.activeElement?.getAttribute("aria-label") === `Problem: ${text}`, diagnostic), "Tab must focus the problem warning next to the name")
  await page.locator('[data-slot="tooltip-content"]').filter({ hasText: diagnostic }).waitFor()
  return "Keyboard check passed (Tab exposes full description and problem tooltips)"
}
