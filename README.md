# React + TypeScript + Vite + shadcn/ui

This is a template for a new Vite project with React, TypeScript, and shadcn/ui.

## Adding components

To add components to your app, run the following command:

```bash
npx shadcn@latest add button
```

This will place the ui components in the `src/components` directory.

## Using components

To use the components in your app, import them as follows:

```tsx
import { Button } from "@/components/ui/button"
```

## Rust workspace and CLI

`crates/core` owns shared application logic. `crates/cli` owns the `unelos`
executable; `src-tauri` owns the desktop adapter. Neither adapter depends on the
other. Default Cargo commands build/test core and CLI without desktop dependencies.

```sh
cargo run -p unelos-cli -- --help
cargo run -p unelos-cli -- config show
cargo run -p unelos-cli -- --config settings.toml --json config show
UNELOS_LOG_LEVEL=info cargo run -p unelos-cli -- config show
cargo run -p unelos-cli -- config show --log-level debug
```

Optional `settings.toml`:

```toml
log_level = "warn"
```

Settings precedence: defaults → explicit TOML file → `UNELOS_*` environment →
explicit CLI flags. `log_level` defaults to `warn`; allowed values are `off`,
`error`, `warn`, `info`, `debug`, and `trace`. `UNELOS_LOG_LEVEL` and `--log-level`
override it. Unknown settings are rejected. No config file is discovered
automatically; `--config` must name an existing UTF-8 TOML file exactly. Inspection
contains only safe settings; diagnostics never render raw config source contents.

Successful results go to stdout. Diagnostics and operational events go to stderr.
`--json` emits a settings object on stdout, application errors as
`{"error":{"code":"invalid_settings","message":"..."}}` on stderr, and
operational events as JSON lines. Application errors contain no ANSI styling.
Clap-generated help, version, and argument/usage errors retain standard text even
with `--json`. Broken output pipes terminate quietly.

Exit codes: `0` success, `2` invalid arguments/settings, `1` operational failure.

### Local Skill resolution (T03)

`unelos_core::resolve_skills(root, ordered_ids)` reads only selected local
`.unelos/skills/<id>/SKILL.md` files, deduplicating IDs in first-occurrence order.
Each result contains its ID, absolute directory, and unchanged UTF-8 content.
Resources remain available through that directory; they are not injected.
IDs must be nonempty, not `.` or `..`, and contain no `/`, `\`, `:`, or NUL.

No CLI Skill command exists yet. Terminal QA exercises real temporary files:

```sh
cargo test --locked -p unelos-core --test skills -- --nocapture
```

Expect five passing checks covering order/content/resources, missing files,
unreadable files, invalid IDs, and working-directory-independent roots.

### Rust checks

```sh
cargo install cargo-nextest --locked
cargo fmt --all -- --check
cargo clippy --locked -p unelos-core -p unelos-cli --all-targets -- -D warnings
cargo nextest run --locked -p unelos-core -p unelos-cli
cargo test --locked -p unelos-core --doc
```

For the desktop adapter, use `bun run tauri dev` or `bun run tauri build` with the
platform's Tauri prerequisites installed. Rust artifacts and `Cargo.lock` now
live at the workspace root. `reqwest` is reserved in workspace dependencies for
shared HTTP operations when a real command needs them; no HTTP wrapper or
placeholder operation is included.
