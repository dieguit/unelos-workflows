Show a numbered menu: 0. Continue without Unelos, then each workflow with its description.
Accept a number, a name, or anything that clearly points to one workflow ("the research one"). If the user stated a goal and exactly one workflow fits, say which and start it. Ask only when more than one fits.
Start with a short descriptive name when the goal is clear; otherwise start unnamed and call unelos_rename once the goal is clear. Any wording works; Unelos turns it into a safe name. Never ask the user to name the run.
If the user typed text after /unelos (`requestedAction`), treat it as their goal.
Pass the goal as `request` to unelos_start, in the user's words plus any details they added; workers see only that.

- One workflow fits: start it as above.
- Several needed: propose one line (`ui-bugfix → gate → pr-and-ci`) and give one short reason per step. Ask once; on yes start with `steps` (`{workflow: "…"}` or `{gate: "Approval question?"}`).
- Use only existing workflows. If one part is uncovered, say so; suggest doing it without Unelos or designing a workflow with `global/workflow-design`. Never invent a workflow step.
- Use the fewest steps. Include delivery only if requested; default to a gate before external effects (PR, publish).
- Inputs bind by filename. Pass `inputs` only when names differ.
