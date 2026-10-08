# Native delivery fixture

Build and inspect from the repository root:
```sh
cargo build --locked -p unelos-cli
target/debug/unelos --project examples/opencode-delivery inspect check-guide --json
target/debug/unelos --project examples/opencode-delivery inspect check-guide --node review --json
```
These commands do not create a run. Copy this example (including `.unelos`) into
a disposable project for native checks; do not run acceptance in this source tree.
Install the [packaged integration](../../integrations/opencode/README.md) there.

Use native `/unelos`, explicitly select `check-guide`, and supply the brief:
“Review the setup guide for engineers who are new to this project.” Observe the
controller starting/naming the run, writing ledger `## Brief`, reading the guide
and selected outline Skill, and writing the declared outline. Test labels are
benign markers, not special instructions. The unselected Skill must stay absent.

The operator—not the delivery hook—then confirms `complete outline` once and
reads the review packet. Code prepares the exact worker payload with the installed
`executorText(packet, brief)` and sends it to one native child with explicit Agent
and the configured/parent-current model. Check its actual checklist read and report.
This controlled child check is not production dispatch/question relay (T16).

Compact/restart only the private server and explicitly activate a fresh controller.
Compare core state/identities, paths and reports; verify ledger brief recovery.
Operator may then confirm `complete review` once to inspect finished recovery.
Never use ledger prose or a worker return as gate consent/progression authority.

Evidence checker: `node integrations/opencode/test/evidence.mjs EVIDENCE_DIRECTORY`
from repository root. Required evidence format is described in that script.
Raw captures, credentials, run state and servers belong outside the repo.
