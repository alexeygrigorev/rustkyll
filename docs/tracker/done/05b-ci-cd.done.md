# Issue 05b: CI/CD Pipeline

## Description

Set up GitHub Actions CI/CD pipeline to run on every push and pull request. The pipeline should build the project, run tests, check formatting, and run clippy.

## Dependencies

- Issue 01 (project setup -- need something to build)

## Scope

- `.github/workflows/ci.yml` with:
  - Trigger on push to main and on pull requests
  - Rust stable toolchain
  - `cargo build`
  - `cargo test`
  - `cargo clippy -- -D warnings`
  - `cargo fmt --check`
  - Cache cargo registry and target directory for faster builds
- Optionally: test on multiple OS (ubuntu, macos) if straightforward

## Notes

- Keep it simple -- just the standard Rust CI checks
- The `datatalksclub.github.io/` directory is gitignored so integration tests that reference it will be skipped in CI (they use guards like `if !dir.exists() { return; }`)
- No `.github/workflows/` directory exists yet; this issue creates it from scratch

## Acceptance Criteria

- [ ] `.github/workflows/ci.yml` exists and is valid YAML
- [ ] Workflow triggers on push to `main` and on pull requests
- [ ] Workflow uses Rust stable toolchain (via `dtolnay/rust-toolchain` or equivalent)
- [ ] Workflow runs `cargo build` and fails the pipeline if it errors
- [ ] Workflow runs `cargo test` and fails the pipeline if any test fails
- [ ] Workflow runs `cargo clippy -- -D warnings` and fails on warnings
- [ ] Workflow runs `cargo fmt --check` and fails if code is not formatted
- [ ] Cargo registry and target directory are cached (via `actions/cache` or `Swatinem/rust-cache`)
- [ ] `cargo build` succeeds locally before merging
- [ ] `cargo test` passes locally (integration tests that depend on `datatalksclub.github.io/` skip gracefully when the directory is absent)
- [ ] `cargo clippy -- -D warnings` passes locally
- [ ] `cargo fmt --check` passes locally

## Test Scenarios

### Manual: Workflow file validation
- Verify `.github/workflows/ci.yml` parses as valid YAML (e.g., `serde_yaml` or an online validator)
- Verify the workflow file contains the expected keys: `name`, `on`, `jobs`
- Verify trigger configuration includes `push` (branches: main) and `pull_request`

### Manual: Local CI parity
- Run `cargo build` locally and confirm it succeeds
- Run `cargo test` locally and confirm all tests pass (skipping those guarded by missing directories)
- Run `cargo clippy -- -D warnings` locally and confirm no warnings
- Run `cargo fmt --check` locally and confirm no formatting issues

### Unit: Workflow file structure (optional, lightweight)
- A test that reads `.github/workflows/ci.yml`, parses it as YAML, and asserts:
  - The `on` key includes `push` and `pull_request` triggers
  - The `jobs` section exists and contains at least one job
  - The job steps include `cargo build`, `cargo test`, `cargo clippy`, and `cargo fmt`
  - A caching step is present (step name or uses contains "cache")
- This test should be in `tests/` or `src/` and can use `serde_yaml` (already a dependency)

### CI: End-to-end (after push)
- Push the workflow to a branch, open a PR, and verify GitHub Actions runs all four checks
- Verify the pipeline passes (green checkmark)
