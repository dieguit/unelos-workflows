# Contributing to Unelos

Thanks for your interest in Unelos. This guide covers how to propose changes, set
up a development environment, and get a pull request merged.

## Before you start

- **Bugs and small fixes:** open a pull request directly, or an issue first if
  you are unsure about the fix.
- **Features and larger changes:** open an issue to discuss them before writing
  code. Unsolicited large PRs are likely to be closed.
- **Unlikely to be merged:** drive-by refactors, formatting-only changes,
  speculative abstractions, and new dependencies without a clear need.

## Prerequisites

- [Rust](https://rustup.rs/) (stable, with `rustfmt` and `clippy`)
- [Node.js](https://nodejs.org/) 24+ and [pnpm](https://pnpm.io/) (version pinned
  in `package.json` via `packageManager`)
- [cargo-nextest](https://nexte.st/): `cargo install cargo-nextest --locked`
- For the desktop app only: the
  [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your
  platform. On Debian/Ubuntu:

  ```sh
  sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev patchelf
  ```

Then install frontend dependencies:

```sh
pnpm install
```

## Project layout

```text
crates/core/         shared application logic (used by CLI and desktop)
crates/cli/          the `unelos` command-line tool
src-tauri/           Tauri desktop backend (thin adapter over core)
src/                 React frontend (TanStack Router file-based routes)
integrations/opencode/  OpenCode plugin (TypeScript, embedded into the CLI)
integrations/shared/    CLI contract types shared by integrations
examples/            example workflow packages
```

Business logic belongs in `core`; the CLI and Tauri adapters stay thin and never
depend on each other. Frontend code is colocated with its route. The full rules
are in [AGENTS.md](AGENTS.md); please read it before structural changes.

Default Cargo commands build only `core` and `cli`, so Rust work does not need
the desktop dependencies.

## Running locally

```sh
# CLI
cargo run -p unelos-cli -- --help
cargo run -p unelos-cli -- --project path/to/project status

# Desktop app (builds the CLI sidecar first, then starts Vite on :3000)
pnpm tauri dev

# Frontend only, in a browser (Tauri commands are unavailable)
pnpm dev
```

`pnpm tauri dev` runs `scripts/build-sidecar.sh`, which builds `unelos-cli` and
copies it to `src-tauri/binaries/` for the app to bundle.

For harness setup and runtime behavior, see the
[OpenCode integration](integrations/opencode/README.md).

## Checks

Run these before opening a pull request. The Rust checks match CI.

```sh
# Rust
cargo fmt --all -- --check
cargo clippy --locked -p unelos-core -p unelos-cli --all-targets -- -D warnings
cargo nextest run --locked -p unelos-core -p unelos-cli
cargo test --locked -p unelos-core --doc

# TypeScript (frontend and integrations)
pnpm lint
pnpm typecheck
pnpm test
```

Format frontend code with `pnpm format`.

## Tests

- Core integration tests live in `crates/core/tests/`, with fixtures in
  `crates/core/tests/fixtures/`. Run one file with
  `cargo nextest run -p unelos-core --test <name>`.
- TypeScript tests are Vitest `*.test.ts` files colocated with the code they
  cover, in `src/` and `integrations/`.
- Integrations ship as raw TypeScript with no build step. Use only erasable
  syntax (no enums or namespaces) and share CLI contract types through
  `integrations/shared/protocol.ts` with `import type`.
- New logic needs a test. Bug fixes should include a test that fails without the
  fix.

## Pull requests

- Keep each PR focused on one change, and link the issue it addresses.
- Use [Conventional Commits](https://www.conventionalcommits.org/) for titles,
  e.g. `feat: add skill table`, `fix(cli): reject empty run names`.
- Describe what changed and how you verified it. Include screenshots for UI
  changes.
- Do not edit generated files such as `src/routeTree.gen.ts`.
- Make sure all checks pass.

## AI-assisted contributions

AI tools are welcome, but you are responsible for every line you submit. Review
and understand the code, run the checks yourself, and disclose substantial AI
assistance in the PR description. Low-effort generated PRs will be closed.

## License and sign-off

Unelos is licensed under the Functional Source License
([FSL-1.1-ALv2](LICENSE.md)): the source is public and free to
use for non-competing purposes, and each release converts to Apache 2.0 two
years after publication.

By contributing, you agree your contribution is licensed under the same terms.
Certify this with the [Developer Certificate of Origin](https://developercertificate.org/)
by signing off each commit:

```sh
git commit -s -m "fix: ..."
```

## Reporting bugs and security issues

- **Bugs:** open a GitHub issue with steps to reproduce, expected and actual
  behavior, your OS, and the output of `unelos --version`.
- **Security vulnerabilities:** do not open a public issue. Report privately
  through GitHub's "Report a vulnerability" button on the repository's Security
  tab.
