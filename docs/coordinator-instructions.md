# Coordinator instructions

The authoritative coordinator policy lives in
[`crates/core/src/instructions/`](../crates/core/src/instructions/README.md):
`controller.md` contains invariants; `controller-*.md` contains state-specific
and resume blocks. Rust embeds these files in execution packages.
