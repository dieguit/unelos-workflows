# Building blocks

How to decide what each piece is, and how to write good ones.

## Task or workflow?

A **task** is one unit of work done by one agent, ending in one outcome (and its
output artifacts). A **workflow** exists only when something must be routed
between steps:

| Need | Use |
|---|---|
| Run checks and report | task |
| Reproduce a bug | task |
| Fix, re-check, fix again until green | workflow (loop) |
| Ask the user to approve something | human gate inside a workflow |
| Backend and UI lanes at the same time | workflow (parallel node) |
| A reusable multi-step procedure used by several workflows | child workflow (`type: workflow`) |

If you are tempted to write "then, if X, go back and…" inside task
instructions, that is routing: make it a workflow and let Unelos enforce it.

## Splitting tasks

Outcomes are fixed per task, so split a task when the same word would mean
opposite things. Reproducing a bug and verifying a fix both drive a browser,
but a "failed" reproduction is a bad sign (no bug found) while a failed
verification means the fix is wrong. Two tasks (`ui-reproduce`: reproduced /
not-reproduced / blocked; `ui-verify`: pass / fail / blocked) sharing one agent
keep routing honest.

## Outcomes

- Name outcomes for what the next step needs to know, not for the work done:
  `ui-only` / `rust-touched` routes to the right checks; `done` routes nowhere.
- Give checking tasks a `blocked` outcome separate from `fail`. "I could not
  check" (server down, MCP missing, mock incomplete) must never look like a
  pass, and it is not something a fixer can fix: route it to a human.
- A task with a single next step needs no outcomes.

## Writing task instructions

- Say what to read (bound inputs, transition feedback on re-entry), what to do,
  what to write (the bound output, with its sections), and how to choose each
  outcome. In that order.
- Describe the output as sections the next step consumes. A fixer that gets
  "Steps / Expected vs actual / Scope" from a reproduction fixes every affected
  screen, not just the reported one.
- Say what happens on re-entry: "On re-entry, read the feedback and update X in
  place". Loops depend on it.
- Repeatable commands belong in the task verbatim (the exact CI commands), not
  as "run the tests". The model is for judgment; anything repeatable should be
  a command it runs.
- Ask for evidence: exact command, exit status, first error with file:line,
  screenshot paths. A report without evidence cannot be trusted by the next step.

## Agents

An agent is a role, reused across tasks.

- **Report-only roles** (QA, check runner, reviewer, recon) say so explicitly:
  never edit code, never fix, never retry a failing check to make it pass.
  `tools` only lists required tools; it does not restrict anything, so the rule
  must be in the instructions.
- **Doer roles** (fixer, implementer) get the project's quality rules: root
  cause first, fix shared code once, smallest change, test logic changes, do
  not commit unless the workflow says so.
- **The main-conversation role** (`execution: main`) is not a separate agent:
  the orchestrator (the user's session) takes that role for the step, on the
  session's model and tools. So a main agent sets no `model` or `mcps` (they
  are ignored, and validation warns). It talks to the user, writes what was
  agreed into an artifact so later steps need no chat history, and does not
  implement. Conversation approval is not gate approval.
- **Keep the orchestrator thin**: only the orchestrator talks to the user, so
  make main steps the conversations (intake, questions, agreeing a contract)
  and nothing else. Drafting specs and plans, checks, fixes, reviews and
  summaries go to subagents with the model the job needs; the user can then run
  the session itself on a cheaper model. A subagent that needs the user returns
  NEEDS_CONTEXT with a question: the orchestrator asks and continues the same
  subagent, without completing the step. Use a human gate instead when the user
  must approve a result, not answer a question.
- **Model by job**: running commands and reporting needs a fast model;
  visual judgment and fixing need a balanced one; design and final review need
  a strong one. Use model aliases so users can remap them.
- **MCPs only where needed**: a browser MCP on the QA agent, a design-tool MCP
  on the recon agent. Every MCP on an agent costs context on every step.
- Every agent ends with "never advance Unelos beyond your own task".

## Skills

Put knowledge in a skill when two or more tasks or agents need it:

- **Project dev skill**: how to run the app, mock or test modes, routes, shared
  components to reuse, where designs live, what evidence to record.
- **Pitfall skill**: what has broken CI before and how to spot it in a diff.
  Give it to both the checker (to detect) and the fixer (to avoid).

Keep task instructions generic and let the skill carry project detail; the same
task then works in another project with a different skill.

## MCPs

Project MCPs live in `.unelos/mcps/<id>.yaml`. If the command contains a path
that only exists on one machine, define it as a global MCP
(`~/.config/unelos/mcps/<id>.yaml`) and reference it as `global/<id>`, so the
repository stays portable.

## Project or global?

Definitions specific to a repository (its checks, its app, its workflows) live
in the project's `.unelos/` and are committed: they document how the team
works and serve as examples. Go global only for pieces that are the same in
every project (a generic `pr-and-ci` loop, a research workflow) or that are
machine-specific (an MCP with a local path). A global workflow can only use
global pieces, so anything project-specific it needs must come from the project
workflow that calls it.
