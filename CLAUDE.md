# Project: Gauntlet Week 1

## Testing Ground Rules

- All tests go in dedicated `*_test.rs` files (e.g., `camera_test.rs`, `doc_test.rs`), never inline in the module source.
- Non-test code must have limited scope per function — each function does one thing.
- State is passed as parameters, not hidden behind struct internals or globals. Functions should be pure where possible so tests can construct exact inputs.
- Tests should be exhaustive: cover happy paths, edge cases, boundary conditions, and error cases.

## Safety Rules

- No panic-capable code in non-test code. No `.unwrap()`, `.expect()`, `panic!()`, or `todo!()` in library/production code.
- Use `Result`, `Option` combinators, or safe defaults instead.
- `todo!()` stubs are only acceptable as temporary placeholders that must be replaced before the module is considered implemented.
- Test code may use `.unwrap()` freely.

## Workflow Rules

- After completing any feature or scope of work, always run in order: `cargo fmt`, `cargo clippy`, `cargo test`, then auto-commit the changes.
- Do not ask for permission to commit — just do it after passing all checks.

## Code Conventions

- Workspace layout: root `Cargo.toml` with members `server` and `canvas`.
- All crates use edition 2024, rust-version 1.90.
- Clippy pedantic warnings enabled; see `clippy.toml` and `[lints.clippy]` in each crate.
- `rustfmt.toml` shared conventions: max_width 120, edition 2024.
