// App shell check. Run against the dev server (http://localhost:3000/) by pasting
// this whole expression into browser DevTools, or via Playwright MCP browser_evaluate
// as `() => <expression>`. Throws on first failed assertion.
(async () => {
  const assert = (condition, message) => {
    if (!condition) throw new Error(message)
  }
  const settle = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
  const sidebar = document.querySelector('aside[aria-label="Project sidebar"]')
  const main = document.querySelector('main')
  assert(sidebar && main, 'Shell landmarks missing')
  assert(document.querySelectorAll('main').length === 1, 'Expected one main landmark')
  assert(sidebar.getBoundingClientRect().width === 240, 'Sidebar must be 240px')
  const buttons = [...sidebar.querySelectorAll('button[data-section]')]
  assert(buttons.length === 5, 'Expected five sections')
  assert(main.querySelector('h1')?.textContent === 'Workflows', 'Default page must be Workflows')
  for (const button of buttons) {
    button.click()
    await settle()
    assert(button.getAttribute('aria-current') === 'page', 'Selected state missing')
    assert(sidebar.querySelectorAll('[aria-current="page"]').length === 1, 'Multiple selected sections')
    assert(main.querySelector('h1')?.textContent === button.textContent, 'Heading does not follow selection')
  }
  const project = sidebar.querySelector('button[aria-disabled="true"]')
  assert(project, 'Unwired project selector must be aria-disabled')
  project.click()
  await settle()
  assert(!document.querySelector('[role="menu"]'), 'Sample project must not open a menu')
  const toggle = sidebar.querySelector('footer button[data-theme-toggle]')
  assert(toggle, 'Theme toggle missing from sidebar footer')
  assert(toggle.nextElementSibling?.dataset.section === 'settings', 'Theme toggle must sit above Project settings')
  const html = document.documentElement
  const wasDark = html.classList.contains('dark')
  const storedTheme = localStorage.getItem('theme')
  toggle.click()
  await settle()
  assert(html.classList.contains('dark') !== wasDark, 'Theme toggle did not switch theme')
  toggle.click()
  await settle()
  assert(html.classList.contains('dark') === wasDark, 'Theme toggle did not switch back')
  if (storedTheme === null) localStorage.removeItem('theme')
  else localStorage.setItem('theme', storedTheme)
  const footer = sidebar.querySelector('footer')
  const before = footer.getBoundingClientRect().top
  const filler = document.createElement('div')
  filler.style.height = '2000px'
  main.append(filler)
  try {
    main.scrollTop = 1000
    await settle()
    assert(main.scrollTop > 0, 'Content must scroll')
    assert(footer.getBoundingClientRect().top === before, 'Footer moved while scrolling')
    assert(document.scrollingElement.scrollTop === 0, 'Outer page scrolled')
    assert(document.scrollingElement.scrollWidth <= innerWidth, 'Horizontal page overflow')
  } finally {
    filler.remove()
    main.scrollTop = 0
    buttons[0].click()
    await settle()
  }
  console.log('App shell check passed')
})()
