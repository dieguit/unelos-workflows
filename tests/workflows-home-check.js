// Workflows home check. Run against the dev server at /workflows (browser dev mock data)
// via DevTools or Playwright MCP browser_evaluate as `() => <expression>`.
;(async () => {
  const assert = (c, m) => {
    if (!c) throw new Error(m)
  }
  const settle = () =>
    new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))
  const main = document.querySelector("main")
  const search = main.querySelector('input[type="search"]')
  const count = () => main.querySelector("[data-workflow-count]")?.textContent
  const rows = () => [...main.querySelectorAll("tbody tr[data-row]")]
  const type = async (value) => {
    const set = Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value"
    ).set
    set.call(search, value)
    search.dispatchEvent(new Event("input", { bubbles: true }))
    await settle()
  }
  const key = async (target, k) => {
    target.dispatchEvent(
      new KeyboardEvent("keydown", { key: k, bubbles: true })
    )
    await settle()
  }

  assert(main.querySelector("h1")?.textContent === "Workflows", "Heading")
  assert(search, "Search input missing")
  assert(count() === "4 workflows", `Initial count: ${count()}`)
  assert(rows().length === 4, "Expected 4 rows")
  assert(
    main.querySelector("button[data-new-workflow]")?.disabled,
    "New workflow must be disabled"
  )
  assert(
    rows()[2].textContent.includes("Agent reference missing"),
    "Problem badge on Review change"
  )

  await type("BUG")
  assert(
    rows().length === 1 && count() === "1 of 4 workflows",
    "Case-insensitive name filter"
  )
  await type("parallel slices")
  assert(
    rows().length === 1 && rows()[0].textContent.includes("Refactor module"),
    "Description filter"
  )
  await type("review-change.yaml")
  assert(rows().length === 1, "File filter")

  await type("deploy")
  assert(
    rows().length === 0 && count() === "0 of 4 workflows",
    "No results count"
  )
  assert(
    main.textContent.includes("No workflows match “deploy”"),
    "No results message"
  )
  ;[...main.querySelectorAll("button")]
    .find((b) => b.textContent === "Clear search")
    .click()
  await settle()
  assert(
    rows().length === 4 && search.value === "",
    "Clear search restores list"
  )

  await type("x")
  await key(search, "Escape")
  assert(search.value === "" && rows().length === 4, "Esc clears")

  document.body.focus()
  await key(document.body, "/")
  assert(document.activeElement === search, '"/" focuses search')
  await key(search, "/")
  assert(
    document.activeElement === search,
    '"/" inside input does not steal focus'
  )

  rows()[0].focus()
  await key(rows()[0], "ArrowDown")
  assert(document.activeElement === rows()[1], "ArrowDown moves focus")
  await key(rows()[1], "ArrowUp")
  assert(document.activeElement === rows()[0], "ArrowUp moves focus")

  const header = main.querySelector("thead")
  assert(
    getComputedStyle(header.querySelector("th")).position === "sticky",
    "Sticky header"
  )
  console.log("Workflows home check passed")
  return "Workflows home check passed"
})()
