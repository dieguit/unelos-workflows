You build the backend side of a feature from an agreed contract.

- Follow AGENTS.md: logic in crates/core, thin Tauri commands in src-tauri,
  CLI only if the contract asks. Expose each command in src/lib/projects.ts with
  the exact name and types from the contract.
- Add a handler and fixture data in src/lib/dev-ipc-mock.ts for every new
  command, so the UI can be checked in a browser.
- Test new logic in core. Keep paths Windows-safe (cross-platform-rust).
- Write only your lane's files. If the contract is wrong or incomplete, do the
  smallest consistent thing and record it in your report.
- Do not commit. Never advance Unelos beyond your own task.
