# Issue 58: Cross-platform binary builds

## Problem

rustkyll currently only builds for the developer's local platform. To be useful as a Jekyll replacement, it needs pre-built binaries for all major platforms so users can download and run without installing Rust.

## Goal

Set up a GitHub Actions release workflow that compiles and publishes rustkyll binaries for:

- Linux x86_64 (amd64)
- Linux aarch64 (arm64)
- macOS x86_64 (Intel)
- macOS aarch64 (Apple Silicon)
- Windows x86_64

## Approach

1. Create `.github/workflows/release.yml` triggered on git tags matching `v*` (e.g. `v0.1.0`)
2. Use a matrix strategy with native runners where available (`ubuntu-latest` for linux-amd64, `macos-latest` for darwin-arm64, `macos-13` for darwin-amd64, `windows-latest` for windows-amd64) and cross-compilation for linux-arm64
3. For Linux arm64: use the `cross` tool or install the `aarch64-unknown-linux-gnu` target with appropriate linker
4. Build in release mode (`cargo build --release`)
5. Package binaries with platform-specific naming: `rustkyll-{os}-{arch}` (plus `.exe` for Windows)
6. Create a GitHub Release and upload all binaries as assets
7. Each binary should be verified to run `--version` on its target platform (where CI runner matches)

## Dependencies

- Issue 05b (CI/CD) -- done

## Acceptance Criteria

- [ ] File `.github/workflows/release.yml` exists and is valid YAML
- [ ] Workflow triggers on push of tags matching `v*` (e.g. `v0.1.0`, `v1.2.3`)
- [ ] Workflow does NOT trigger on regular pushes to main or pull requests (that is ci.yml's job)
- [ ] Matrix builds produce binaries for all 5 targets:
  - `rustkyll-linux-amd64`
  - `rustkyll-linux-arm64`
  - `rustkyll-darwin-amd64`
  - `rustkyll-darwin-arm64`
  - `rustkyll-windows-amd64.exe`
- [ ] Binaries are built in release mode (`--release`)
- [ ] A GitHub Release is created with all 5 binaries attached as assets
- [ ] On native runners (linux-amd64, darwin-arm64, windows-amd64), the built binary is verified by running `./rustkyll --version` and confirming it exits 0
- [ ] The workflow uses caching (`Swatinem/rust-cache` or equivalent) to speed up builds
- [ ] The existing `ci.yml` workflow is NOT modified or broken
- [ ] The release workflow also runs `cargo test` on at least one platform (linux-amd64) before building all targets, to avoid releasing broken code

## Test Scenarios

Since this is a CI/CD workflow issue (no Rust library code changes), testing is focused on workflow correctness rather than `cargo test` unit tests.

### Workflow file validation
- The YAML file parses without errors (`python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"` or similar)
- The workflow has the correct `on:` trigger (push tags `v*`)
- The matrix includes all 5 target triples

### Local verification
- `cargo build --release` succeeds on the developer's machine (sanity check)
- The release binary runs `rustkyll --version` and prints the version from Cargo.toml

### CI verification (after pushing a test tag)
- Push a tag like `v0.1.0-rc1` and verify the workflow runs
- All 5 matrix jobs complete successfully (green checkmarks)
- The GitHub Release page shows all 5 binary assets
- Download at least one binary and confirm it runs `--version` correctly
- Delete the test tag and release after verification

### Edge cases
- Workflow handles the case where a tag is pushed but code does not compile (should fail the release, not create a partial release)
- Binary names are consistent and do not have extra extensions or prefixes

## Notes

- The `cross` tool (https://github.com/cross-rs/cross) is the standard way to cross-compile Rust for linux-arm64 from an x86_64 runner if a native arm64 runner is not available. Alternatively, GitHub now offers `ubuntu-latest` with ARM runners -- check availability.
- For macOS, `macos-latest` gives Apple Silicon (arm64) and `macos-13` gives Intel (x86_64).
- Windows arm64 is not included in scope since it is a very niche target.
- Consider using `actions/upload-artifact` during the build and `softprops/action-gh-release` or the `gh` CLI to create the release and attach assets.
