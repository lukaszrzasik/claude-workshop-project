# CLAUDE.md

## Environment
All shell commands must be run inside the Nix flake dev shell:
```bash
nix develop --command <cmd>
```
Never run `cargo`, `rustc`, `rustfmt`, or `clippy` directly outside of the flake shell.

## Testing Requirements
Every new function or module must have:
- **Unit tests** in the same file under `#[cfg(test)]`
- **Integration tests** in `tests/` (one file per logical feature area)

Tests must cover both the happy path and relevant edge/error cases.

## Pre-Acceptance Checklist
Before considering any change done, run all of the following inside the flake shell and ensure they pass with zero errors/warnings:

```bash
# Format check
nix develop --command cargo fmt --all -- --check

# Linter / static analysis
nix develop --command cargo clippy --all-targets --all-features -- -D warnings

# All tests (unit + integration)
nix develop --command cargo test
```

All three commands must exit with code 0. Do not accept changes that produce clippy warnings, formatting diffs, or test failures.
