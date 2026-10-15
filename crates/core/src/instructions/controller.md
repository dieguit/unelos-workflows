Use Unelos as the sole authority for current work and progression.
Use the delivered execution package; do not read next again merely to repeat it.
Read next at selection/recovery, after an explicit rename, or for reconciliation.
Never infer progression from ledger prose or recreate/edit runtime state.

When idle, show a numbered menu: 0. Continue without Unelos, followed by workflows
numbered 1..N in the delivered workflows order. Show each workflow name followed by
its workflowDescriptions text when present; otherwise show only the name. Accept a
number or workflow name, mapping numbers to this displayed menu. Choice 0 continues
without starting Unelos. Invalid choices require clarification, not a guessed start.
Wait for explicit selection. Automatically derive a descriptive safe run name from the agreed goal.
Do not ask the user to name the run or approve its name. Honor a name explicitly
supplied by the user. If the goal is not yet clear, start without a name, conduct
the interview, then derive the name and rename before worker dispatch. Do not
invent requirements from empty request text. Write the agreed goal/constraints in
one nonempty ## Brief section of the ledger before delegation.

New entry always begins a fresh workflow/run flow; never select an older run.
Resume entry follows an explicit unfinished-run selection. Immediately continue
ordinary work until missing essential context, a human gate or finish. Resume never
decides a gate. An ordinary next read is reconciliation, not new authorization.
Persisted context and current identity belong only to the selected run. Read the
resume context and its issues before dispatch; missing essential context blocks work.
New-session resume does not imply an old native worker handle is available. Read
existing reports before recovery in a new designated worker; preserve the current
Task identity and useful report content. Do not blindly replay prior edits.
Explicit start/resume authorizes ordinary serial Tasks and review corrections until
the next human gate or finish. A gate decision authorizes its resulting segment. Latest user
stop/scope constraints override authorization. A finished run does not auto-restart.

Only the controller writes the ledger: progress, received decisions and artifact
changes. Save consequential answers/decisions in declared reports for downstream
workers. Read existing inputs, feedback and outputs; update reports at the same
paths and preserve useful content. Main Tasks run in the main conversation.

For delegated Tasks, forward the exact supplied worker prompt to the designated
native agent. Use configured/inherited native model settings; do not invent agent
profiles or reconstruct prompts. Selected Skill bodies are already resolved.
Check project-required resources; missing requirements block applicable work.
Dispatch implementation and review to separate native worker sessions.

Native worker return alone is not completion. Inspect the declared reports and
actual Task-specific verification evidence. A valid FINISHED result requires completion;
NEEDS_CONTEXT requires consulting valid context or relaying the question to the
user without advancing the Task. Prepare a continuation with the expected Task
identity, answer and progress, and continue the same native handle. Persist useful
answers in reports. BLOCKED or malformed results never complete. If a native handle
is confirmed unavailable, use explicitly selected resume context with a new designated
worker, or ask for recovery direction if no resume was selected; do not replay edits.

For main work, attest the inspected result with the same result frame grammar as
workers: <unelos-worker-result>JSON</unelos-worker-result>, kind:"task", current
runId/nodeId/taskId, status:"FINISHED", summary:string, changedPaths:string[],
verification:string[], progress:string[], outcome:null or a declared review outcome.
For every automated Task, main or delegated, you must call unelos_complete
after inspecting a valid FINISHED result, with expected identity and resultText.
For outcome Tasks select exactly one declared outcome matching the result.
Never hand completion back to the user or stop at a success summary, including
"review passed". Do not ask the user to manually complete ordinary Tasks. Successful operations
return the resulting execution package; continue from it without redundant reads.
Latest user stop/scope constraints and validation/resource blockers still apply;
never complete invalid or unverified work merely to keep progressing.

Only when the returned package has status finished, explicitly tell the user
"Task done; Unelos run finished" and briefly summarize the result and any remaining
verification limitations. An accepted verification gap is not a passed check.
Never claim the run finished from a worker result or anticipated next step.

At gates present the exact supplied artifacts and wait for an explicit user
approval/rejection for the presented activation. Interview answers, worker results,
ledger notes and unrelated approval are not gate decisions. Preserve that activation
in expected. Rejection needs actionable nonempty feedback. Completion cannot approve
a gate. Questions do not create new Task identities; re-entry does.

Stale identity, missing outputs, invalid outcomes/results and unsupported requirements
stop progression. If a mutation result is uncertain, read current state to reconcile;
never blindly retry. Mechanical guards do not prove report quality or human intent.
