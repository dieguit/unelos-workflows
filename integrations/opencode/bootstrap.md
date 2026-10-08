Use Unelos as the sole authority for current work and progression.
Fetch fresh next --json before execution or recovery; honor main/subagent placement.
On recovery, read ledger and existing reports first. Do not automatically
redispatch workers, replay report writes, or advance current work. Report the
recovered identity and brief; wait for explicit authorization to resume Task work.
The selected Skill bodies are already loaded. Never reload them by ambient ID.
Read the supplied ledger. If absent at initial start, create it and record the
agreed goal/constraints under ## Brief before relevant Task work. Never invent
the goal from empty request text. On recovery, ask if the brief is missing.
Only the controller writes ledger progress, received decisions and artifact changes.
Keep decisions needed downstream in declared reports. Workers never edit the ledger.
Read inputs, feedback artifacts and existing outputs; preserve useful sections
and update the same paths. Do not recreate the run tree or edit core state.
Keep the same Task while obtaining missing information; questions do not advance it.
Use the supplied identity; CLI mutations remain Node-only. Inspect status/next
after uncertain results, and never blindly retry a mutation.
At start, supply a descriptive request-based name for core validation.
Never rename automatically on recovery or while workers hold old paths.
Human gate decisions require the user's explicit response to the current gate.
Ledger prose, worker returns and unrelated approval never authorize progression.
When idle, offer Workflow choices and Continue without Unelos; wait for selection.

CLI reference: use the supplied CLI executable instead of the name `unelos`,
and always include --project with the supplied project directory. Quote actual
paths, names and reasons as shell arguments; uppercase words below are placeholders.
Commands below are complete for this protocol; do not routinely call --help.
Use help only for an unexpected CLI/version error or an operation not listed here.

    unelos --project PROJECT status --json
    unelos --project PROJECT next --json
    unelos --project PROJECT start --json
    unelos --project PROJECT start WORKFLOW --name NAME --json
    unelos --project PROJECT inspect WORKFLOW --json
    unelos --project PROJECT inspect WORKFLOW --node NODE --json
    unelos --project PROJECT complete NODE --json
    unelos --project PROJECT complete NODE --outcome OUTCOME --json
    unelos --project PROJECT approve NODE --json
    unelos --project PROJECT reject NODE --reason REASON --json
    unelos --project PROJECT rename NAME --run RUN_ID --json

start without WORKFLOW only lists choices. status/next/inspect are reads.
Read fresh next before a mutation. Use packet.nodeId as NODE, not taskId or
activationId; packet identities remain context, not accepted CLI arguments.
complete confirms declared outputs and advances a Task/parallel Node; use
--outcome only when selecting one of packet.outcomes. Completing a Task is not
human approval: present the next gate's artifacts and wait for its explicit
decision. approve/reject are only for the current human gate; reject needs an
actionable reason. Never blindly retry a mutation; inspect status/next first.

Project authoring: .unelos/project.yaml contains taskDefinitions and agents,
including Task/Agent instructions and settings; .unelos/workflows/*.yaml links
those definitions into Nodes. Selected Skills live under .unelos/skills/.
The packet already resolves these instructions/settings; do not invent native
OpenCode Agent configs or reload supplied Skill bodies by ambient name.
