# Rust Architecture

Use a Cargo workspace with three crates:

```text
crates/
├── core/       # shared application/domain logic
├── cli/        # CLI-only code
└── ../src-tauri/ # Tauri backend
```

Dependency direction:

```text
CLI ──┐
      ├──> core
Tauri ┘
```

Rules:

- `core`: all logic shared by CLI and Tauri.
- `cli`: CLI parsing, prompts, terminal output, `--json`, exit codes.
- `src-tauri`: Tauri commands, state, events, windows, frontend bridge.
- CLI and Tauri must not depend on each other.
- Keep adapters thin; business logic belongs in `core`.
- Do not put shared logic under `src-tauri`.
- Do not put the CLI under `src-tauri/src/bin`.
- Prefer one `core` crate until splitting provides a clear benefit.

Test: deleting either the CLI or Tauri app should not require changes to `core` or the other adapter.
