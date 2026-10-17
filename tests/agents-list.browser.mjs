import assert from "node:assert/strict"
import { mkdirSync } from "node:fs"
import { join } from "node:path"
import { pathToFileURL } from "node:url"

// Browser check for the Agents list. Requires the dev server on :3000 (`bun run dev`).
// Playwright MCP browser_run_code_unsafe cannot import modules: pass this body inline.
// Node: PLAYWRIGHT_MODULE=<playwright or playwright-core entry> CHROME_PATH=<chromium> [SCREENSHOT_DIR=<dir>] node tests/agents-list.browser.mjs
// (no Playwright dependency is added to this repo)

const A = "/home/dieguit/projects/unelos/unelos-app"
const B = "/home/dieguit/work/api-gateway"
const engineer = { name: "engineer", model: "provider/fast", skills: ["rust", "typescript"], mcps: ["github"], problem: null }
const reviewer = { name: "reviewer", model: null, skills: [], mcps: [], problem: null }
const broken = { name: "broken", model: null, skills: [], mcps: [], problem: "model: expected a string" }
const search = (page) => page.getByRole("searchbox", { name: "Search agents" })
const cell = (page, name) => page.getByRole("cell", { name, exact: true })
const warning = (page, problem) => page.getByRole("img", { name: `Problem: ${problem}`, exact: true })

// Test-only wrapper around the dev mockIPC invoke. Rejects unexpected commands and
// unknown list_agents roots; supports one-shot failure and holding a root's response.
function installAgentsIPC() {
  const internals = window.__TAURI_INTERNALS__
  const inner = internals.invoke
  const allowed = new Set([
    "project_state",
    "open_project",
    "pick_project",
    "remove_recent",
    "list_workflows",
    "list_agents",
    "plugin:opener|reveal_item_in_dir",
  ])
  const ipc = (window.__agentsIPC = { agents: {}, calls: [], fail: null, held: null })
  internals.invoke = async (cmd, args, options) => {
    if (!allowed.has(cmd)) throw `agents-list check: unexpected IPC command ${cmd}`
    if (cmd !== "list_agents") return inner(cmd, args, options)
    ipc.calls.push(args.root)
    if (ipc.fail) {
      const message = ipc.fail
      ipc.fail = null
      throw message
    }
    if (!(args.root in ipc.agents)) throw `agents-list check: no fixture for ${args.root}`
    const data = structuredClone(ipc.agents[args.root])
    if (ipc.held && args.root in ipc.held)
      return new Promise((resolve) => ipc.held[args.root].push(() => resolve(data)))
    return data
  }
}

async function open(page, baseUrl, agents) {
  await page.goto(`${baseUrl}/agents`)
  await page.getByRole("heading", { name: "Agents", exact: true }).waitFor()
  await page.evaluate(installAgentsIPC)
  await setAgents(page, agents)
  await refresh(page)
}

async function setAgents(page, agents) {
  await page.evaluate((fixtures) => void (window.__agentsIPC.agents = fixtures), agents)
}

async function refresh(page) {
  const before = await page.evaluate(() => window.__agentsIPC.calls.length)
  await page.evaluate(() => window.dispatchEvent(new Event("focus")))
  await page.waitForFunction((n) => window.__agentsIPC.calls.length > n, before)
}

async function hold(page, root) {
  await page.evaluate((r) => void (window.__agentsIPC.held = { ...window.__agentsIPC.held, [r]: [] }), root)
  await refresh(page)
  await page.waitForFunction((r) => window.__agentsIPC.held[r].length > 0, root)
}

async function release(page, root) {
  await page.evaluate((r) => {
    const ipc = window.__agentsIPC
    ipc.held[r].splice(0).forEach((resolve) => resolve())
    delete ipc.held[r]
  }, root)
}

async function setFail(page, message) {
  await page.evaluate((m) => void (window.__agentsIPC.fail = m), message)
}

async function waitForResultStatus(page, text) {
  await page.waitForFunction((t) => document.querySelector('[data-list-count]')?.textContent === t, text)
}

async function checkSearchAndClear(page, baseUrl) {
  await open(page, baseUrl, { [A]: [engineer, reviewer] })
  const box = search(page)
  await page.getByRole("columnheader", { name: "Name", exact: true }).waitFor()
  await waitForResultStatus(page, "2 agents")
  await page.evaluate(() => (window.__statusNode = document.querySelector('[data-list-count]')))
  await box.fill(" ENGINE ")
  await cell(page, "engineer").waitFor()
  assert.equal(await cell(page, "reviewer").count(), 0)
  await waitForResultStatus(page, "1 of 2 agents")
  await box.fill("github")
  await page.getByText("No agents match “github”", { exact: true }).waitFor()
  await waitForResultStatus(page, "0 of 2 agents")
  assert.equal(
    await page.evaluate(() => window.__statusNode === document.querySelector('[data-list-count]')),
    true,
    "result status must stay mounted so changes are announced",
  )
  await page.getByRole("button", { name: "Clear search" }).click()
  await cell(page, "reviewer").waitFor()
  await waitForResultStatus(page, "2 agents")
  assert.equal(await page.getByRole("button", { name: "New Agent" }).count(), 0)
  assert.equal(await box.inputValue(), "")
  assert.equal(await box.evaluate((el) => el === document.activeElement), true, "Clear search must restore search focus")
}

async function checkKeyboard(page, baseUrl) {
  await open(page, baseUrl, { [A]: [engineer, reviewer] })
  const box = search(page)
  const focused = () => box.evaluate((el) => el === document.activeElement)
  await page.getByRole("heading", { name: "Agents", exact: true }).click()
  await page.keyboard.press("/")
  assert.equal(await focused(), true, "/ must focus search outside editable elements")
  await page.keyboard.type("eng")
  await cell(page, "engineer").waitFor()
  await page.keyboard.press("Escape")
  assert.equal(await box.inputValue(), "", "first Escape clears query")
  assert.equal(await focused(), true, "first Escape keeps focus")
  await page.keyboard.press("Escape")
  assert.equal(await focused(), false, "Escape on empty query blurs")
  await box.fill("")
  await box.focus()
  await page.keyboard.press("/")
  assert.equal(await box.inputValue(), "/", "/ types normally inside search")
  await page.keyboard.press("Escape")
  await page.keyboard.press("Escape")
  await cell(page, "engineer").click()
  assert.equal(new URL(page.url()).pathname, "/agents", "row click must not navigate")
  assert.equal(await page.getByRole("link", { name: "engineer" }).count(), 0, "names are not links")
}

async function checkProblemRow(page, baseUrl) {
  await open(page, baseUrl, { [A]: [broken, engineer, reviewer] })
  const icon = warning(page, broken.problem)
  await icon.waitFor()
  assert.equal(await page.getByText("broken", { exact: true }).isVisible(), true, "broken agent keeps its row")
  assert.equal(await page.getByRole("heading", { name: /with problems/ }).count(), 0, "no top notice")
  await icon.hover()
  await page.locator('[data-slot="tooltip-content"]').filter({ hasText: broken.problem }).waitFor()
  await page.mouse.move(0, 0)
  await search(page).fill("eng")
  await cell(page, "engineer").waitFor()
  assert.equal(await icon.count(), 0, "search filters problem rows like any other row")
  await search(page).fill("")
}

async function checkEmptyStates(page, baseUrl) {
  await open(page, baseUrl, { [A]: [] })
  await page.getByText("No agents yet", { exact: true }).waitFor()
  assert.equal(await search(page).count(), 0, "no search without agents")
}

async function checkErrorAndRetry(page, baseUrl) {
  await open(page, baseUrl, { [A]: [reviewer] })
  await cell(page, "reviewer").waitFor()
  await setFail(page, "boom")
  await refresh(page)
  await page.getByText("Couldn’t load agents: boom", { exact: false }).waitFor()
  await page.getByRole("button", { name: "Retry", exact: true }).click()
  await cell(page, "reviewer").waitFor()
}

async function checkFocusInvalidation(page, baseUrl) {
  await open(page, baseUrl, { [A]: [engineer] })
  await cell(page, "engineer").waitFor()
  await setAgents(page, { [A]: [engineer, { ...reviewer, name: "auditor" }] })
  await refresh(page)
  await cell(page, "auditor").waitFor()
}

async function checkProjectSwitch(page, baseUrl) {
  await open(page, baseUrl, {
    [A]: [broken, engineer],
    [B]: [{ ...broken, name: "b-broken", problem: "B notice" }, { ...reviewer, name: "docs-writer" }],
  })
  await cell(page, "engineer").waitFor()
  await hold(page, A)
  await refresh(page)
  await page.evaluate((path) => window.__TAURI_INTERNALS__.invoke("open_project", { path }), B)
  await hold(page, B)
  await refresh(page)
  await page.locator('[role="status"]', { hasText: "Loading agents" }).first().waitFor()
  assert.equal(await cell(page, "engineer").count(), 0, "project A rows must not show while B loads")
  assert.equal(await warning(page, broken.problem).count(), 0, "project A problem must not show while B loads")
  await release(page, B)
  await cell(page, "docs-writer").waitFor()
  await warning(page, "B notice").waitFor()
  await release(page, A)
  await page.waitForTimeout(300)
  assert.equal(await cell(page, "engineer").count(), 0, "stale project A rows must not appear")
  assert.equal(await warning(page, broken.problem).count(), 0, "stale project A problem must not appear")
  assert.equal(await cell(page, "docs-writer").isVisible(), true)
  assert.match(await page.getByRole("button", { name: /api-gateway/ }).innerText(), /api-gateway/)
}

// Returns descendants whose box extends outside root horizontally (clipping/overflow).
function horizontalOverflow(locator) {
  return locator.evaluate((root) => {
    const box = root.getBoundingClientRect()
    return [root, ...root.querySelectorAll("*")]
      .filter((el) => {
        const r = el.getBoundingClientRect()
        return r.width > 0 && (r.left < box.left - 1 || r.right > box.right + 1)
      })
      .map((el) => `${el.tagName} ${el.textContent.slice(0, 40)}`)
  })
}

async function checkErrorWrap(page, baseUrl) {
  await page.setViewportSize({ width: 768, height: 900 })
  const longError = "boom-" + "e".repeat(150)
  await open(page, baseUrl, { [A]: [reviewer] })
  await cell(page, "reviewer").waitFor()
  await setFail(page, longError)
  await refresh(page)
  const message = page.getByText("Couldn’t load agents", { exact: false })
  await message.waitFor()
  assert.deepEqual(await horizontalOverflow(message.locator("..")), [], "error notice must not clip horizontally")
  await page.setViewportSize({ width: 1440, height: 900 })
}

async function checkLongStrings(page, baseUrl, width, screenshotDir) {
  await page.setViewportSize({ width, height: 900 })
  const long = {
    name: "long-" + "n".repeat(70),
    model: "provider/" + "m".repeat(90),
    skills: Array.from({ length: 10 }, (_, i) => `skill-${i}-${"s".repeat(20)}`),
    mcps: ["mcp-" + "z".repeat(80)],
    problem: null,
  }
  await open(page, baseUrl, { [A]: [long, reviewer] })
  await cell(page, long.name).waitFor()
  const report = await page.evaluate(() => {
    const table = document.querySelector("table")
    return {
      heights: [...document.querySelectorAll("tbody tr")].map((r) => Math.round(r.getBoundingClientRect().height)),
      texts: [...document.querySelectorAll("tbody td")].map((td) => td.textContent),
      tableOverflow: table.scrollWidth - table.parentElement.clientWidth,
      pageOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    }
  })
  assert.equal(report.heights[0], report.heights[1], `${width}px: long row must stay one line`)
  if (width >= 1440) assert.ok(report.tableOverflow <= 1, `${width}px: table wider than card ${report.tableOverflow}px`)
  assert.ok(report.pageOverflow <= 0, `${width}px: page horizontal overflow ${report.pageOverflow}px`)
  for (const t of [long.name, long.model, long.skills.join(", "), long.mcps[0]])
    assert.equal(report.texts.includes(t), true, `${width}px: full text kept in DOM`)
  if (screenshotDir) await page.screenshot({ path: join(screenshotDir, `agents-list-${width}.png`), fullPage: true })
  await page.setViewportSize({ width: 1440, height: 900 })
}

async function checkRowKeyboard(page, baseUrl) {
  await open(page, baseUrl, { [A]: [engineer, reviewer] })
  await cell(page, "engineer").waitFor()
  const activeRow = () =>
    page.evaluate(() => {
      const el = document.activeElement
      return el?.tagName === "TR" ? el.querySelector("td").textContent : null
    })
  await search(page).click()
  await page.keyboard.press("Tab")
  assert.equal(await activeRow(), "engineer", "Tab from search must reach first row")
  assert.equal(
    await page.evaluate(() => getComputedStyle(document.activeElement).boxShadow !== "none"),
    true,
    "focused row must show a ring",
  )
  await page.keyboard.press("ArrowDown")
  assert.equal(await activeRow(), "reviewer", "ArrowDown moves to next row")
  await page.keyboard.press("ArrowUp")
  assert.equal(await activeRow(), "engineer", "ArrowUp moves to previous row")
}

export async function checkAgentsList(page, { baseUrl = "http://localhost:3000", screenshotDir = "/tmp/opencode/agents-list" } = {}) {
  mkdirSync(screenshotDir, { recursive: true })
  await page.setViewportSize({ width: 1440, height: 900 })
  await checkSearchAndClear(page, baseUrl)
  await checkKeyboard(page, baseUrl)
  await checkProblemRow(page, baseUrl)
  await checkEmptyStates(page, baseUrl)
  await checkErrorAndRetry(page, baseUrl)
  await checkFocusInvalidation(page, baseUrl)
  await checkProjectSwitch(page, baseUrl)
  await checkLongStrings(page, baseUrl, 1440, screenshotDir)
  await checkLongStrings(page, baseUrl, 768, screenshotDir)
  await checkRowKeyboard(page, baseUrl)
  await checkErrorWrap(page, baseUrl)
  await open(page, baseUrl, { [A]: [broken, engineer, reviewer] })
  await cell(page, "engineer").waitFor()
  await page.screenshot({ path: join(screenshotDir, "agents-list-default-1440.png"), fullPage: true })
  return "agents-list browser check passed"
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const playwright = await import(process.env.PLAYWRIGHT_MODULE ?? "playwright").catch(() => import("playwright-core")).catch(() => null)
  if (!playwright) {
    console.error("Requires a resolvable playwright or playwright-core package; or run via Playwright MCP.")
    process.exit(2)
  }
  const browser = await playwright.chromium.launch({ executablePath: process.env.CHROME_PATH })
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })
    console.log(await checkAgentsList(page, { baseUrl: process.env.BASE_URL ?? "http://localhost:3000", screenshotDir: process.env.SCREENSHOT_DIR }))
  } finally {
    await browser.close()
  }
}
