---
name: test-driven-development
description: Write the failing test first, watch it fail, then the minimal code to pass.
---

> Adapted from superpowers' test-driven-development skill (MIT, Copyright (c) 2025 Jesse Vincent). License: LICENSE-superpowers in the superpowers-dev package.

# Test-Driven Development (TDD)

## Overview

Write the test first. Watch it fail. Write minimal code to pass.

**Core principle:** If you didn't watch the test fail, you don't know if it tests the right thing.

**Violating the letter of the rules is violating the spirit of the rules.**

## When to Use

**Always:** new features, bug fixes, refactoring, behavior changes.

**Exceptions (return NEEDS_CONTEXT if the task doesn't say):** throwaway prototypes, generated code, configuration files.

Thinking "skip TDD just this once"? Stop. That's rationalization.

## The Iron Law

```
NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
```

Write code before the test? Delete it. Start over.

**No exceptions:**
- Don't keep it as "reference"
- Don't "adapt" it while writing tests
- Don't look at it
- Delete means delete

Implement fresh from tests. Period.

## Red-Green-Refactor

RED (write failing test) → verify it fails correctly (wrong failure: back to RED) → GREEN (minimal code) → verify it passes, all green (fails: back to GREEN) → REFACTOR (stay green) → next test.

### RED - Write Failing Test

Write one minimal test showing what should happen.

Good: `test('retries failed operations 3 times', ...)` — clear name, real behavior, one thing; the operation is a plain function that throws twice then returns, and the test asserts the result and the attempt count.

Bad: `test('retry works', ...)` — vague name, asserts a mock was called, tests the mock not the code.

**Requirements:**
- One behavior
- Clear name
- Real code (no mocks unless unavoidable)
- Name the production change that would make the test fail, before writing it
- Assert on real behavior, never on mock behavior
- Keep test-only code in test utilities, out of production classes
- Understand a dependency's side effects before mocking it

### Verify RED - Watch It Fail

**MANDATORY. Never skip.** Run the test.

Confirm:
- Test fails (not errors)
- Failure message is expected
- Fails because feature missing (not typos)

**Test passes?** You're testing existing behavior. Fix test.

**Test errors?** Fix error, re-run until it fails correctly.

### GREEN - Minimal Code

Write simplest code to pass the test. A retry loop of three attempts is enough; an options object with `maxRetries`, `backoff` and `onRetry` is over-engineered (YAGNI).

Don't add features, refactor other code, or "improve" beyond the test.

### Verify GREEN - Watch It Pass

**MANDATORY.** Run the test.

Confirm:
- Test passes
- Other tests still pass
- Output pristine (no errors, warnings)

**Test fails?** Fix code, not test.

**Other tests fail?** Fix now.

**"Other tests" means the project's suite, not just your file.** A green run of the test you wrote is not a green suite. Before you call the change done, run the project's test command (bare `pytest`, `npm test`, `cargo test` — whatever the repo uses) even when your task named only one test file. A scope statement in your task bounds the deliverable, not your verification. Any failure that run shows — including one you didn't cause — goes in your report by name; a red test you watched scroll past and didn't mention is a report falsified by omission.

### REFACTOR - Clean Up

After green only: remove duplication, improve names, extract helpers. Keep tests green. Don't add behavior.

### Repeat

Next failing test for next feature.

## Good Tests

| Quality | Good | Bad |
|---------|------|-----|
| **Minimal** | One thing. "and" in name? Split it. | `test('validates email and domain and whitespace')` |
| **Clear** | Name describes behavior | `test('test1')` |
| **Shows intent** | Demonstrates desired API | Obscures what code should do |

## Common Rationalizations

| Excuse | Reality |
|--------|---------|
| "Too simple to test" | Simple code breaks. Test takes 30 seconds. |
| "I'll test after" | Tests written after pass immediately — which proves nothing. They may test the wrong thing, test the implementation instead of the behavior, or miss the edge case you forgot. You never watched it fail, so you never proved it can catch the bug. |
| "Tests after achieve same goals (spirit not ritual)" | Tests-after answer "what does this do?"; tests-first answer "what should this do?" Tests written after are biased by the code you already wrote. |
| "Already manually tested" | Manual testing is ad-hoc: no record, no way to re-run it. "Worked when I tried it" ≠ comprehensive. |
| "Deleting X hours is wasteful" | Sunk cost fallacy. Keeping code you can't trust is the waste. |
| "Keep as reference, write tests first" | You'll adapt it. That's testing after. Delete means delete. |
| "Need to explore first" | Fine. Throw away exploration, start with TDD. |
| "Test hard = design unclear" | Listen to test. Hard to test = hard to use. |
| "TDD will slow me down" | TDD IS the pragmatic path: catches bugs before commit, prevents regressions. |
| "Manual test faster" | Manual doesn't prove edge cases. You'll re-test every change. |
| "Existing code has no tests" | You're improving it. Add tests for existing code. |

## Red Flags - STOP and Start Over

- Code before test
- Test after implementation
- Test passes immediately
- Can't explain why test failed
- Tests added "later"
- Rationalizing "just this once"
- "I already manually tested it"
- "Tests after achieve the same purpose"
- "It's about spirit not ritual"
- "Keep as reference" or "adapt existing code"
- "Already spent X hours, deleting is wasteful"
- "TDD is dogmatic, I'm being pragmatic"
- "This is different because..."

**All of these mean: Delete code. Start over with TDD.**

## Verification Checklist

Before marking work complete:

- [ ] Every new function/method has a test
- [ ] Watched each test fail before implementing
- [ ] Each test failed for expected reason (feature missing, not typo)
- [ ] Wrote minimal code to pass each test
- [ ] All tests pass
- [ ] Output pristine (no errors, warnings)
- [ ] Tests use real code (mocks only if unavoidable)
- [ ] Edge cases and errors covered

Can't check all boxes? You skipped TDD. Start over.

## When Stuck

| Problem | Solution |
|---------|----------|
| Don't know how to test | Write wished-for API. Write assertion first. Still unclear: return NEEDS_CONTEXT. |
| Test too complicated | Design too complicated. Simplify interface. |
| Must mock everything | Code too coupled. Use dependency injection. |
| Test setup huge | Extract helpers. Still complex? Simplify design. |

## Final Rule

```
Production code → test exists and failed first
Otherwise → not TDD
```
