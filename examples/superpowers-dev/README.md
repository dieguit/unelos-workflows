# superpowers-dev

Take a feature from idea to finished branch: agree a spec, approve a plan, then
build it task by task with a review after every task and a final review of the
whole branch. Use it for real features where you want to sign off on the design
and plan, then let agents do the build without check-ins.

Adapted from [superpowers](https://github.com/obra/superpowers) (MIT, see
`LICENSE-superpowers`).

## How it works

1. **Spec** — the controller reads the project, asks only questions that change what
   gets built, and writes `spec.md`. For a first full draft, a builder-check agent
   lists what a builder would still have to ask, and the spec is revised. You
   approve or reject it.
2. **Plan** — the controller writes `plan.md`: small tasks with exact code and
   commands. You approve or reject it.
3. **Worktree** — work happens in an isolated git worktree, starting from passing tests.
4. **Task loop** — per task: the implementer builds it (test first), the reviewer
   checks it, the controller decides: fix, next task, or escalate. Fix rounds 1–3
   go back to the implementer, rounds 4–5 to an escalation implementer; after that
   the controller makes a ruling and moves on.
5. **Final review** — the whole branch is reviewed from the merge-base. Must-fix
   findings get one fix wave.
6. **Finish** — verify, list the rulings made along the way, then you choose: merge
   locally, push and open a PR, keep the branch, or discard.

```text
brainstorm ⇄ spec-check → approve-spec → write-plan → approve-plan → setup-worktree
→ [implement-task → review-task → advance]* (⇄ escalate-task)
→ final-review ⇄ fix-final → finish → done
```

## Dependencies

- `git` — worktrees, diffs, merge.
- A git remote you can push to, only if you pick "push and open a PR" at the end.

## Agents

- `sp-controller` — runs in your session: spec, plan, worktree, loop decisions, finish.
- `sp-builder-check` — reads the spec cold and lists what a builder would have to ask.
- `sp-implementer` — builds one task or fix round; also does the final fix wave.
- `sp-implementer-escalation` — takes fix rounds 4–5 with fresh context; give it a
  stronger model than the implementer.
- `sp-task-reviewer` — reviews each task and each fix round.
- `sp-final-reviewer` — reviews the whole branch; give it your strongest model.

No agent sets a model, so all use your session model unless you change it.

## Skills

- `brainstorming` — controller, writing the spec.
- `writing-plans` — controller, writing the plan.
- `using-git-worktrees` — controller, setting up the worktree.
- `sdd-controller` — controller, loop rules: preflight, fix caps, rulings, next step.
- `finishing-a-development-branch` — controller, the finish step.
- `test-driven-development`, `verification-before-completion`,
  `receiving-code-review`, `systematic-debugging` — both implementers.
- `code-review` — task and final reviewers.
