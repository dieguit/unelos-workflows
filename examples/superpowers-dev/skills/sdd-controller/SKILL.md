---
name: sdd-controller
description: Controller rules for the task loop: preflight, fix-loop cap, rulings, and outcome selection.
---

> Adapted from superpowers' subagent-driven-development skill (MIT, Copyright (c) 2025 Jesse Vincent). License: LICENSE-superpowers in the superpowers-dev package.

# Subagent-Driven Development: Controller

You coordinate; the workflow dispatches. A fresh implementer per task writes `implementation-report.md`, a task reviewer (spec compliance + code quality) writes `review.md`, and a broad final reviewer writes `final-review.md`; a single final fix wave writes `final-fix-report.md`. Your job is to read those artifacts, keep `progress.md` accurate, rule on conflicts, and choose the outcome. Never fix findings yourself — your context stays clean for coordination, and controller fixes skip review.

**Continuous execution:** Do not pause to check in with your human partner between tasks. Execute all tasks from the plan without stopping. The only reasons to stop are the four named below, or all tasks complete. "Should I continue?" prompts and progress summaries waste their time — they asked you to execute the plan, so execute it.

**Rulings, not stalls.** A running plan does not wait on a human. Conflicts,
ambiguities, plan defects, a cap you would have asked to exceed — decide
them. The spec is the binding authority, the plan is its argument, and your
judgment settles what neither answers. Record every decision in the run artifacts as
`Ruling: <what you decided> — <why> — <what it costs if wrong>`, and keep
going. A wrong ruling costs rework your human partner can see and undo; a
session parked on a question costs their whole day and buys nothing.

Four things stop you, and only these: an irreversible or destructive
operation; a security-sensitive action; a side effect outside this worktree
that norms say you ask about first (a merge, a push to a shared branch, a
publish); and a plan so broken that every path forward is a guess. For those,
stop and ask.

## Node map

- **setup-worktree**: after the worktree skill, run the preflight scan on `plan.md`; write the table and every `Ruling:` into `worktree.md`.
- **advance**: read the latest section of `review.md` and your existing `progress.md` (initialize it from `plan.md` on first visit: `Current: Task 1`, task count, `Phase: tasks`). Update `progress.md`, then choose exactly one outcome:
  - `fix`: spec ❌, any Critical/Important, or a ⚠️ item you confirm as a real gap, and fix round R < 5. Write `Task N: fix round R/5 (…)` and the open findings verbatim under `## Open findings`.
  - At R = 5 with findings open: adjudicate each (`Task N: parked — <finding> — Ruling: <why> — cost if wrong: <…>`, or rule the smallest unblocking change and record it under `## Carry forward`), then treat the task as complete.
  - `next_task`: task complete and tasks remain. Write `Task N: complete (commits a..b, review clean | K parked)`, set `Current: Task N+1`, clear `## Open findings`.
  - `all_done`: last task complete; set `Phase: done`.
  - Minor findings never trigger `fix`: write `Task N: minor (deferred): <one-liner>`.
- **advance-final**: read `final-review.md`. Must-fix findings and no `## Re-review of fix wave` section → `final_fix`. Clean, or the re-review section exists (the single fix wave is spent) → write every residual's `Ruling:` to `final-rulings.md`, then `finish`.

## Setup preflight

Conversation memory does not survive compaction. `progress.md` is your recovery map: tasks with a `Task <N>: complete` line are DONE; a task whose last line is a fix round is mid-loop. After compaction, trust `progress.md` and `git log` over your own recollection.

Read `plan.md` once, note its context and Global Constraints. Read the Spec it names: the spec is the authority the plan argues from, and conflicts inside the plan resolve against it. A plan with no reachable spec gets a note saying so — rulings made without one are provisional.

Before Task 1, scan the plan once for conflicts, writing down what you checked as you check it:

- tasks that contradict each other or the plan's Global Constraints
- anything the plan explicitly mandates that the review rubric treats as a
  defect (a test that asserts nothing, verbatim duplication of a logic block)

The scan's output is a table, not a verdict. One row for every pair of tasks
that share a file or an interface: the two tasks, what one produces against
what the other consumes, and what you found. One row for every task: whether
its own text agrees with itself — the tests it specifies against the code it
specifies, the files it creates against the files it later touches. "The scan
is clean" without those rows is not a scan you ran.

Write the table to `worktree.md`. Rule on everything you find before execution
begins — each finding against the plan text that mandates it — and record
each ruling beside its row. If the scan is clean, proceed without comment.
The review loop remains the net for conflicts that only emerge from
implementation.

## Handle the implementation report

`implementation-report.md` carries one of four statuses:

- **DONE:** proceed on the review verdict.
- **DONE_WITH_CONCERNS:** read the concerns. If they are about correctness or scope, treat them as open findings (`fix`). If they are observations (e.g., "this file is getting large"), note them and proceed.
- **NEEDS_CONTEXT:** the implementer lacked information. Supply it (or rule on the ambiguity) under `## Carry forward` in `progress.md` and choose `fix` so the next round carries it.
- **BLOCKED:** assess the blocker. Context problem → supply context. Task too large → rule a smaller scope. Plan itself wrong → rule on the correction and record it. In every case carry the ruling forward and choose `fix`.

**Never** ignore an escalation or send the same work back unchanged. If the implementer said it's stuck, something needs to change.

## Reading the review

Never accept a review missing either verdict — spec compliance AND task quality are both required. Implementer self-review never replaces the task review.

The task reviewer may report "⚠️ Cannot verify from diff" items — requirements that live in unchanged code or span tasks. These do not block the rest of the review, but you must resolve each one yourself before marking the task complete: you hold the plan and cross-task context the reviewer lacks. If you confirm an item is a real gap, treat it as a failed spec review — it enters the fix loop with the other findings.

## The fix loop

The loop triggers when the review reports spec ❌, any Critical or Important
finding, or a ⚠️ item you confirmed as a real gap.

Two routes leave it immediately:

- Record Minor findings in `progress.md` as you go
  (`Task <N>: minor (deferred): <one-liner>`); the final
  whole-branch review triages which must be fixed
  before merge. A roll-up nobody reads is a silent discard. Minor findings
  never enter the loop.
- A finding labeled plan-mandated — or any finding that conflicts with
  what the plan's text requires — is yours to rule on first: weigh the finding
  against the plan text, decide with the spec as the binding authority, and
  record the ruling before you act on it. Do not dismiss the finding because
  the plan mandates it, and do not send a fix that contradicts the plan
  without a recorded ruling.

Everything else enters the loop. A fix round is one fix pass plus one
scoped re-review. Five rounds maximum per task. The re-review is scoped:
each finding is verdicted ADDRESSED or NOT ADDRESSED, and only the fix diff is
checked for new breakage. New Critical/Important breakage in the fix diff joins
the open findings. Out-of-scope observations become deferred minors — they
never extend the loop.

**The breaker.** When round 5's re-review still leaves findings open, stop
looping. Adjudicate each open finding yourself — you hold the plan and
the cross-task context the reviewer lacks:

- **The reviewer is wrong, or the point is contestable:** park it —
  `Task <N>: parked — <finding> — Ruling: <why the code stands>`. The final
  review sees both sides.
- **Real, but nothing downstream builds on it:** park it the same way, with
  a ruling that says it's real and deferred.
- **Real and load-bearing** — a later task builds on it, or it reveals a
  plan defect: rule on the smallest change that unblocks the dependent work,
  record it as `Task <N>: Ruling: <finding> — <what you decided and why>`
  under `## Carry forward`, so it reaches the next task. Parking a structural failure
  silently lets every dependent task build on it. Stop only when the defect
  leaves every path forward a guess.

Adjudicate only at the cap. Adjudicating earlier to end a loop is
pre-judging with a different name. Every adjudication is a `progress.md` entry —
a silent discard is forbidden.

## Complete the task

When the review comes back clean — or every open finding is parked with a
ruling at the cap — write the completion line to `progress.md` in the same
update as your other bookkeeping:

- `Task <N>: complete (commits <base7>..<head7>, review clean)`
- `Task <N>: complete (commits <base7>..<head7>, <K> parked)` after a
  tripped breaker

Never move to the next task while the review has open Critical/Important
issues that are neither fixed nor parked-with-ruling at the cap.

## Final review

The final whole-branch review reads the deferred-minor and parked lines in `progress.md` and triages which must be fixed before merge.

If it returns must-fix findings, ONE fix wave handles the complete findings list — not one fixer per finding — followed by exactly one scoped re-review of that wave. Adjudicate any residual findings as in the breaker: park with rulings, or rule on the load-bearing ones and record what you decided in `final-rulings.md`. Only the four stop classes above stop you here. There is no second fix wave — residual load-bearing findings surface to your human partner when the branch is finished.

## Finish: Rulings I made

Collect every line containing `Ruling:` — from `worktree.md`, `progress.md` and `final-rulings.md`: preflight rulings, parked findings, breaker adjudications, all of them — under "Rulings I made", in the order you made them, each with what it costs if wrong. The list is exhaustive: if an artifact holds a ruling, the list holds it. That list is the only place the decisions you took on your human partner's behalf reach them — they read it and rework whatever you got wrong. A ruling that goes unreported was a decision made in secret.

## Common Rationalizations

| Excuse | Reality |
|--------|---------|
| "Close enough on spec compliance" | Reviewer found spec gaps = not done. Fix or hit the cap and adjudicate — those are the only exits. |
| "I'll fix it myself, a fix round is overhead" | Controller fixes pollute your context and skip review. Choose `fix`. |
| "One more round will converge" | Past the cap, rounds don't converge — the failure is structural. Adjudicate and route. |
| "The reviewer will just find something new anyway" | Scoped re-reviews verify fixes; they cannot wander. New findings on untouched code become deferred minors, not loop rounds. |
| "This finding is obviously wrong, I'll drop it" | You adjudicate only at the cap, and every ruling is a recorded entry. Silent discards are forbidden. |
| "The fix was small, skip the re-review" | Unreviewed fixes are how regressions land. Every round ends with a scoped re-review. |
| "Reviews slow the loop down" | The loop without reviews is just unverified churn. Reviews are the loop's brakes and steering. |
| "`progress.md` bookkeeping is overhead" | It is what survives compaction. Controllers without it have redone entire completed task sequences. |
