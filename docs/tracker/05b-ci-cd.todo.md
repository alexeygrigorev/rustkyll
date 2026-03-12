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
- The datatalksclub.github.io/ directory is gitignored so integration tests that reference it will be skipped in CI (they should handle missing directory gracefully)
