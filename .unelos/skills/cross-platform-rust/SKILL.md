---
name: cross-platform-rust
description: Rust pitfalls that pass on Linux but fail CI on Windows or macOS in unelos-app, and how to spot them in a diff.
---

# Cross-platform Rust

CI runs on ubuntu, windows and macos; local runs are Linux only. These broke CI
here before:

| Pitfall | Example | Do instead |
|---|---|---|
| Path built or compared as a string with `/` | `root.join("a/b/c.md").as_str()` compared to a runtime path; `format!("{root}/x")` | Join per segment (`.join("a").join("b")`) or compare `Utf8Path` values; never assert on separators |
| CRLF line endings | Windows checkout turns fixtures into `\r\n`; string asserts or text edits silently no-op | Normalize with `.replace("\r\n", "\n")` before comparing or editing (see `normalize_fixture_text` in `crates/core/tests`) |
| Backslash in JSON/YAML | A Windows path written unescaped into JSON or YAML text | Serialize with serde, never by `format!` |
| Home directory | `HOME` is unset on Windows (`USERPROFILE`) | Use `global_config::home()` (falls back to `USERPROFILE`); in tests set both variables |
| Temp dir symlinks | macOS `/var` is `/private/var`, so canonical paths differ from temp paths | Canonicalize both sides or compare relative paths |
| Executable names | `.exe` suffix on Windows | Use `std::env::consts::EXE_SUFFIX` |
| Newer clippy on CI | CI uses latest stable; a lint can fail there first | Run `rustup update stable` before trusting a local clippy pass |

## Reviewing a diff

Look at the changed Rust lines (`git diff <base>...HEAD -- '*.rs'`) for:

- `"/` or `/"` inside `join(`, `format!(` or `assert_eq!` near a path
- `.as_str()` or `.to_string()` on a path inside an assertion
- `read_to_string` on fixtures without CRLF normalization
- `env::var("HOME")`, `"/tmp"`, `std::env::temp_dir()` with canonical-path asserts
- `format!` building JSON or YAML that contains a path

Report each hit with file:line and which pitfall it matches. A hit is a risk,
not proof: say why it is or is not safe.
