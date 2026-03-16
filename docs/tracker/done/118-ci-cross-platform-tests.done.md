# Issue 118: Add Windows and macOS CI integration tests

## Problem

Integration tests only run on Linux. We need to verify rustkyll builds and runs correctly on Windows and macOS in CI. Platform-specific bugs (like the Unicode panic #78 which was Windows-only) can slip through undetected.

## Goal

Add Windows and macOS jobs to the integration workflow (`integration.yml`). These jobs must:
- Build rustkyll on each platform
- Run the unit test suite
- Clone the DTC site and build it with rustkyll
- Verify the output page count meets the minimum threshold
- Catch platform-specific panics or failures

Only the DTC site is needed for cross-platform testing -- no need to clone all 16 sites on Windows/macOS.

## Approach

Add new jobs to `.github/workflows/integration.yml` using a strategy matrix or separate job definitions for `windows-latest` and `macos-latest` runners. These jobs run alongside the existing Linux integration job (gated by the same `check-changes` job).

Key considerations:

1. **Windows shell compatibility:** The existing `compare-output.sh` script uses bash. GitHub Actions Windows runners include Git Bash, but the script uses `/tmp/` paths. The cross-platform jobs should either:
   - Use `shell: bash` on Windows (which GitHub Actions supports via Git Bash) and adjust temp paths to use `$RUNNER_TEMP` instead of `/tmp/`, OR
   - Inline the validation logic directly in the workflow step (build the site, count HTML files, check threshold) to avoid shell compatibility issues.

   The simpler approach is to inline the validation: build with rustkyll, count `.html` files, compare against a minimum threshold. This avoids bash script portability issues entirely.

2. **Binary path:** On Windows the binary is `rustkyll.exe` and lives under `target/release/rustkyll.exe`. Use `cargo build --release` and reference the binary accordingly per platform.

3. **No Ruby/Jekyll needed:** Cross-platform jobs only build with rustkyll and validate output. No Jekyll comparison is needed.

4. **Same trigger:** These jobs should fire on the same schedule/workflow_dispatch trigger and respect the `check-changes` gate.

## Dependencies

None. The existing `integration.yml` and rustkyll binary are sufficient.

## Acceptance Criteria

- [ ] `.github/workflows/integration.yml` contains jobs for Windows (`windows-latest`) and macOS (`macos-latest`) in addition to the existing Linux job
- [ ] All three platform jobs depend on the existing `check-changes` job (only run if there were recent commits)
- [ ] Each cross-platform job performs these steps:
  - [ ] Checks out the rustkyll repo
  - [ ] Installs Rust stable
  - [ ] Uses `rust-cache` for caching
  - [ ] Runs `cargo build --release`
  - [ ] Runs `cargo test --verbose` (unit tests only, NOT `--ignored`)
  - [ ] Clones the DTC site (shallow clone)
  - [ ] Builds the DTC site with the rustkyll release binary (`rustkyll build --source <dtc-dir> --destination <output-dir>`)
  - [ ] Counts the number of `.html` files in the output directory
  - [ ] Asserts the HTML file count meets a minimum threshold (at least 100, matching `MIN_HTML_FILES` in `compare-output.sh`)
- [ ] The Windows job correctly handles the `.exe` binary extension
- [ ] The macOS job works without platform-specific workarounds (or documents any needed)
- [ ] The existing Linux integration job is unchanged -- it continues to run all 16+ sites and `--ignored` tests as before
- [ ] The workflow file is valid YAML and passes `actionlint` or similar validation (or at minimum has correct syntax)
- [ ] Jobs are triggered on both `schedule` (nightly) and `workflow_dispatch` (manual), same as existing

## Test Scenarios

### Workflow Validation
- Read the modified `integration.yml` and verify it contains jobs for all three platforms (linux, windows, macos)
- Verify the cross-platform jobs depend on `check-changes`
- Verify `cargo test` in cross-platform jobs does NOT use `--ignored` (those are slow integration tests meant only for Linux)
- Verify the existing Linux `integration` job is completely unchanged

### Build Verification (manual / CI run)
- Trigger the workflow manually via `workflow_dispatch` and verify all three platform jobs appear in the Actions UI
- Verify `cargo build --release` succeeds on Windows and macOS
- Verify `cargo test --verbose` passes on Windows and macOS

### DTC Site Build Verification
- Verify the DTC site is cloned on each cross-platform runner
- Verify rustkyll builds the DTC site without panics on Windows and macOS
- Verify the HTML file count output is logged and meets the minimum threshold (>=100 files)
- Verify the job fails if the HTML file count is below the threshold

### No Regression
- Verify the existing Linux job still clones all 16+ sites, runs `--ignored` tests, and runs `compare-output.sh --validate-only` for each site
- Verify no steps were removed or modified in the existing Linux job

## Log

### [SWE] 2026-03-16
- Added `cross-platform` job to `.github/workflows/integration.yml` using a strategy matrix with `windows-latest` and `macos-latest`
- Matrix includes platform-specific binary path (`target/release/rustkyll.exe` for Windows, `target/release/rustkyll` for macOS)
- Job depends on `check-changes` (same gate as existing Linux job)
- Steps: checkout, install Rust, rust-cache, `cargo build --release`, `cargo test --verbose` (no `--ignored`), clone DTC site, build with rustkyll, validate HTML count >= 100
- Uses `shell: bash` and `$RUNNER_TEMP` for cross-platform path compatibility
- Inline validation logic (no dependency on bash scripts)
- Existing Linux `integration` job is completely unchanged
- YAML validated with Python yaml.safe_load
- All existing tests pass: 1487 passed, 0 failed
- Clippy clean, fmt clean
- Files modified: `.github/workflows/integration.yml`
