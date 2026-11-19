# Workflow patterns

Shapes that recur in real projects. Adapt them; do not copy them blindly.

## Bug fix (pilot candidate)

```
intake (main) → bug.md
reproduce (QA) → repro.md
  reproduced → fix
  not-reproduced / blocked → confirm [gate]: approved → fix, rejected → intake
fix (fixer) → fix.md                outcome picks which checks run
checks (check runner)               fail → fix
verify (QA, replays repro.md)       fail → fix, blocked → approve
approve [gate]                      rejected → fix
```

Why: agents fix the reported screen but not its siblings, and claim success
without looking. Reproducing first gives the fixer exact steps and scope, and
replaying the same steps afterwards is an objective check.

## UI feature from a design

```
intake (main): request, design frames, reference screen → brief.md
design recon: frames, states, components to reuse vs new → design.md
  missing → ask the user: design first or build without
contract (main): backend/UI interface agreed → contract.md [gate]
parallel: backend lane | UI lane
checks → verify (QA against design + contract)
visual QA [gate] → rejected → fix lane
consistency sweep: sibling screens behave the same
```

Why: UI work fails on unseen states, inconsistency between screens and
ignored house components. The recon step forces reuse; the gate is where a
human judges visuals.

## Backend task (spec → plan → implement)

```
questions until shared understanding (main)
spec [gate] → plan [gate]
per plan task: implement → review → fix loop
whole-branch review
local CI-equivalent checks + cross-platform review
manual QA steps for a human (short, copy-pasteable)
PR → wait for CI → fix loop
```

Why: CI on other operating systems is where these usually fail; checking
locally before the PR saves a round trip.

## Integration / install

```
recon: how the feature reaches the external tool
contract → parallel lanes
fresh build + install into a sandbox project (prove nothing stale is in use)
live end-to-end run → gate
```

Why: stale installed binaries, plugins and background processes make live
tests lie.

## Research question

```
parallel explorers (different angles) → synthesis → gate ("simpler?")
```

## Review to fix plan

```
independent reviewers (parallel, different models) → synthesis
decision table [gate] → phased fix lanes (each formats only its own files)
```

## Reusable sub-workflows

Small loops shared by several workflows are worth extracting as child
workflows, e.g. `pr-and-ci`: open PR → wait for CI → fail → fix → push → wait.

## Work and delivery apart

End a work workflow at its summary artifact and keep delivery (`pr-and-ci`)
as its own workflow. The user then picks per run: no PR for one-time work,
update the branch's PR, or a new one, and can drive bigger work by running
workflows one by one. Give the first step an optional input (e.g.
`request.md`): a missing input does not block the run; the step uses it when
bound and asks the user otherwise. The same workflow then runs standalone or
as a child that receives its brief.
