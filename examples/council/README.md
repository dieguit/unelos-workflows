# council

Ask a question, get three independent opinions from different models, and one
merged answer. Use it for decisions where a single model's take isn't enough:
architecture calls, risky changes, debugging dead ends, "is this the right approach".

Adapted from the council feature in
[oh-my-opencode-slim](https://github.com/alvinunreal/oh-my-opencode-slim) (MIT, see
`LICENSE-oh-my-opencode-slim`).

## How it works

1. **Assignment** — you and the chair agree the question, context and what a good
   answer looks like. Saved to `question.md`.
2. **Council** — three councillors answer in parallel. Each runs on its own model,
   reads the code but never edits it, and never sees the others' answers.
3. **Synthesis** — the council agent compares the answers, resolves disagreements
   and writes `council-report.md`: the final answer, what each councillor said,
   and the consensus level (unanimous, majority or split).
4. **Approve** — you accept the report or reject it with feedback, which sends the
   question back to the councillors.

```text
assignment → council (alpha | beta | gamma) → synthesis → approve → done
                  ↑                                          │
                  └──────────────── rejected ────────────────┘
```

## Dependencies

None. Councillors use the model aliases `smart`, `balanced` and `fast`; map each to
a different model before installing, otherwise install fails.

## Agents

- `council-chair` — runs the assignment interview.
- `councillor-alpha` (`smart`) — correctness, bugs, edge cases.
- `councillor-beta` (`balanced`) — architecture, boundaries, maintainability.
- `councillor-gamma` (`fast`) — simplicity, performance, what to cut.
- `council` — merges the three answers into the report.

## Skills

- `council-deliberation` — used by every councillor; fixes the report shape
  (answer, reasoning with file references, assumptions, confidence, uncertainties)
  so answers are comparable.
