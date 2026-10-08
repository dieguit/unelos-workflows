# Build and inspect Unelos

From a source checkout with Rust and Cargo installed, run:

```sh
cargo build --locked -p unelos-cli
target/debug/unelos --project examples/opencode-delivery inspect check-guide --json
target/debug/unelos --project examples/opencode-delivery status --json
```

The build creates `target/debug/unelos`. Inspection returns the normalized Workflow
without starting work; status reports `idle` before a run exists. Choose a Workflow
explicitly before starting. The desktop app is not needed for CLI inspection.
