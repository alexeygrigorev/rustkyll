# Issue 01: Project Setup

## Description

Initialize the Rust project with Cargo. Set up the binary crate, directory structure, and basic dependencies (serde, serde_yaml, clap for CLI). The binary should compile and print a "hello world" message.

## Dependencies

None (first issue)

## Scope

- `Cargo.toml` with project metadata and initial dependencies
- `src/main.rs` with basic CLI skeleton (clap)
- `src/lib.rs` as library root
- Project compiles, `cargo test` runs (even if no real tests yet)
- `cargo clippy` and `cargo fmt` pass

## Acceptance Criteria

- [ ] `Cargo.toml` exists at the project root with package name `rustkyl`
- [ ] `Cargo.toml` lists these dependencies: `serde` (with `derive` feature), `serde_yaml`, `clap` (with `derive` feature)
- [ ] `src/main.rs` exists and uses clap to define a CLI with at least a `--help` flag
- [ ] Running `cargo run -- --help` prints usage information without errors
- [ ] `src/lib.rs` exists as the library root (may be minimal but must exist)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` runs and passes (at least 3 meaningful tests -- see test scenarios below)
- [ ] `cargo clippy -- -D warnings` produces no warnings
- [ ] `cargo fmt --check` reports no formatting issues
- [ ] No `unwrap()` calls in library code (`src/lib.rs` and any modules under `src/`)

## Test Scenarios

### Unit: Cargo.toml validity
- Verify the project compiles (implicit via `cargo test` running at all)

### Unit: CLI argument parsing
- Test that the CLI parses valid arguments without error (e.g., a default subcommand or `build` subcommand)
- Test that the CLI rejects unknown flags (clap handles this, but a test confirms the CLI definition is wired up)

### Unit: Library root
- Test that `lib.rs` exposes at least one public item (a module, constant, or function) to confirm the library crate is usable
- A simple smoke test (e.g., a function that returns the version string or project name) to verify `lib.rs` is properly connected

### Integration: Binary runs
- Test that `cargo run -- --help` exits with code 0 (can be a `#[test]` using `std::process::Command`)

## Notes

- The CLI skeleton should anticipate future subcommands (e.g., `build`, `serve`) but only needs to define the top-level structure for now. A `build` subcommand that prints a placeholder message is sufficient.
- Keep dependencies minimal -- only add what is listed in scope. Additional dependencies come in later issues.
