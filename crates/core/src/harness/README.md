# Adding a harness adapter

Implement `HarnessAdapter` in this module and add it to `registry()` (order is API).
Use `DetectOnly` for detection-only entries. Shared logic belongs here, never in CLI/Tauri.
Add isolated environment/temp-directory tests in `tests.rs`. Serialize environment changes.

OpenCode installs only v2 `plugins` entries. JSONC edits use the CST to preserve
foreign entries/comments. Bundled assets use the core crate version as integration
version: bump the workspace version whenever changing these assets.
The stable CLI is shared across integrations and intentionally survives uninstall.
