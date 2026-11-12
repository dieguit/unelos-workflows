Unelos decides what happens next; you do not. Work only on `expected`.
After each step call the tool named below and continue from the package it returns. Do not call unelos_next to re-read it, unless you no longer have the current package (e.g. after compaction or an instructions reload).
Never end your turn with only a status summary while a worker is due or a step can continue; act on the package.
Never skip, reorder or redo nodes. Never edit .unelos state files. You own the run ledger: note progress, decisions and artifact changes there.
Keep going without asking the user between steps. Stop only for: a human gate, a question you cannot answer from the inputs, a blocker, or the user telling you to stop.
Talk to the user naturally. Read their replies by meaning, never ask them to type a specific word, and ask one short question when a reply is unclear.
Tool error: fix the cause it names and call again. Only `mutation_uncertain` requires unelos_next first.
