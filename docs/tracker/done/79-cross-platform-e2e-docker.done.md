# Issue 79: Cross-platform end-to-end testing via Docker (Windows + macOS)

## Problem

We can only test rustkyll on Linux locally. The Unicode panic (#78) was only caught by a user running on Windows. We need automated cross-platform testing that exercises the real binary on real Windows and macOS environments before each release.

## Resources

- https://github.com/dockur/windows -- Windows in Docker (KVM-based, runs real Windows)
- https://github.com/dockur/macos -- macOS in Docker (KVM-based, runs real macOS)

These allow running actual Windows/macOS VMs inside Docker containers on a Linux host. KVM is available on this machine. The `dockurr/windows:latest` image is already pulled.

## Goal

Create a self-contained script that:
1. Builds or downloads the platform-specific rustkyll binary (Windows `.exe`, macOS binary)
2. Spins up dockur/windows and dockur/macos Docker containers
3. Copies the binary and a test site into each container
4. Runs `rustkyll build` inside each container
5. Extracts the output and compares it to Linux output
6. Reports pass/fail per platform

This is a **local, on-demand script** intended to be run by QA before every release. It is NOT a CI workflow (the KVM requirement makes it impractical for GitHub Actions).

## Approach

### Binary acquisition

The script must cross-compile (or use pre-built release binaries) for each target platform:
- Windows: `x86_64-pc-windows-msvc` target (use `cross` for cross-compilation from Linux, or download from a GitHub release)
- macOS: `x86_64-apple-darwin` target (use `cross` or download from a GitHub release)

The script should support two modes:
1. `--binary-dir <path>` -- use pre-built binaries from this directory (expects `rustkyll-windows-amd64.exe` and `rustkyll-darwin-amd64`)
2. `--release <tag>` -- download binaries from the specified GitHub release
3. Default (no flag) -- attempt to cross-compile using `cross`

### Container lifecycle

Each platform test follows this pattern:
1. Start dockur container with appropriate settings (RAM, disk, KVM passthrough)
2. Wait for the OS to boot (dockur containers need time to install/boot Windows/macOS)
3. Copy the binary and test site into the container
4. Run `rustkyll build` inside the container
5. Extract the generated `_site/` output
6. Shut down the container
7. Compare output to Linux baseline

### Test site

Use the DTC site (`datatalksclub.github.io/`) as the test site. The Linux baseline is generated first by running the native Linux binary.

### Output comparison

Compare the file tree and file contents across all 3 platforms. Expected result: identical output (same files, same content). Any differences should be reported with diffs.

## Dependencies

- Issue 78 (Unicode panic fix) -- DONE
- Issue 58 (Cross-platform release workflow) -- DONE
- Requires KVM support on the host machine (`/dev/kvm` must exist)
- Requires Docker installed and running
- `dockurr/windows:latest` image already pulled; `dockurr/macos` may need to be pulled

## Acceptance Criteria

### Script existence and structure

- [ ] A script exists at `scripts/e2e-cross-platform.sh` (or similar) that orchestrates the full cross-platform test
- [ ] The script is executable (`chmod +x`)
- [ ] The script accepts `--binary-dir <path>` to use pre-built binaries
- [ ] The script accepts `--release <tag>` to download binaries from a GitHub release
- [ ] The script accepts `--platform <windows|macos|all>` to test specific platforms (default: `all`)
- [ ] The script prints clear status messages for each phase (building baseline, starting container, copying files, running build, comparing output)
- [ ] The script exits with code 0 on success, non-zero on failure

### Windows testing

- [ ] The script starts a `dockurr/windows` container with KVM passthrough (`--device /dev/kvm`)
- [ ] The script waits for Windows to be ready (boot detection, not just a fixed sleep)
- [ ] The rustkyll Windows binary (`rustkyll.exe`) is copied into the container
- [ ] The DTC site source files are copied into the container
- [ ] `rustkyll.exe build` runs inside the Windows container and completes without panics or errors
- [ ] The generated `_site/` output is extracted from the container back to the host

### macOS testing

- [ ] The script starts a `dockurr/macos` container with KVM passthrough
- [ ] The script waits for macOS to be ready (boot detection)
- [ ] The rustkyll macOS binary is copied into the container
- [ ] The DTC site source files are copied into the container
- [ ] `rustkyll build` runs inside the macOS container and completes without panics or errors
- [ ] The generated `_site/` output is extracted from the container back to the host

### Linux baseline

- [ ] The script generates a Linux baseline by running the native `rustkyll build` on the host
- [ ] The Linux baseline is used as the reference for cross-platform comparison

### Output comparison

- [ ] The script compares the file tree (list of generated files) across all tested platforms against the Linux baseline
- [ ] The script compares file contents (HTML output) across all tested platforms against the Linux baseline
- [ ] Differences are reported clearly with file paths and diffs
- [ ] The script reports the total number of files checked and any mismatches
- [ ] Platform-specific line ending differences (CRLF vs LF) are normalized before comparison (Windows will produce CRLF)

### Documentation

- [ ] A section in the script or a companion document (`docs/cross-platform-testing.md`) explains how to run the tests
- [ ] Prerequisites are documented (Docker, KVM, disk space, RAM requirements)
- [ ] Expected runtime is documented (dockur Windows boot takes several minutes)
- [ ] Troubleshooting tips for common failures (KVM not available, container fails to boot, binary not found)

### Error handling

- [ ] The script cleans up containers on exit (even on failure/Ctrl+C) using a trap
- [ ] The script validates prerequisites before starting (Docker running, KVM available, required images pulled)
- [ ] The script provides clear error messages when prerequisites are missing
- [ ] Timeout handling: if a container fails to boot within a reasonable time (e.g., 15 minutes), the script times out and reports failure

## Test Scenarios

### Manual verification (by QA before release)

These are manual test scenarios because the dockur containers take significant time to boot and require KVM:

#### Scenario 1: Windows end-to-end
- Run: `scripts/e2e-cross-platform.sh --platform windows --binary-dir ./path/to/binaries`
- Verify: Windows container boots, rustkyll.exe runs, site builds, output extracted
- Verify: Output file count matches Linux baseline
- Verify: HTML content matches Linux baseline (after CRLF normalization)
- Verify: No panics in rustkyll.exe output

#### Scenario 2: macOS end-to-end
- Run: `scripts/e2e-cross-platform.sh --platform macos --binary-dir ./path/to/binaries`
- Verify: macOS container boots, rustkyll runs, site builds, output extracted
- Verify: Output file count matches Linux baseline
- Verify: HTML content matches Linux baseline

#### Scenario 3: Full cross-platform comparison
- Run: `scripts/e2e-cross-platform.sh --binary-dir ./path/to/binaries`
- Verify: Both Windows and macOS are tested
- Verify: Summary report shows pass/fail per platform
- Verify: All three platforms produce identical output

#### Scenario 4: Prerequisite validation
- Run the script without Docker running -> verify it exits with a clear error
- Run the script without KVM -> verify it exits with a clear error
- Run the script with `--binary-dir` pointing to empty directory -> verify it exits with a clear error listing missing binaries

#### Scenario 5: Cleanup on failure
- Start the script, then Ctrl+C during container boot
- Verify: containers are stopped and removed (no orphaned dockur containers)
- Run: `docker ps -a | grep -E 'rustkyll|dockur'` -> should return nothing

### Automated unit tests (in Rust test suite)

Since the cross-platform Docker tests are inherently slow and require KVM, no Rust unit tests are needed for this issue. The deliverable is a shell script with manual QA verification. However:

- [ ] The script itself should have a `--dry-run` mode that validates prerequisites and prints what it would do without actually starting containers
- [ ] Running `scripts/e2e-cross-platform.sh --dry-run` should succeed on any machine with Docker installed (even without KVM), verifying the script's logic is sound

## Notes

- dockur/windows containers typically need 4-8 GB RAM and take 5-15 minutes to boot depending on the Windows version
- dockur/macos containers may need more RAM (8+ GB) and can take longer to boot
- The total test run for both platforms could take 30-60 minutes
- This is intentionally a local-only tool, not a CI pipeline -- KVM is not available on standard GitHub Actions runners
- The script should be idempotent: running it twice should work without manual cleanup

## Log

### [SWE] 2026-03-15

- Created `scripts/e2e-cross-platform.sh` -- cross-platform e2e test orchestrator
- Created `docs/cross-platform-testing.md` -- comprehensive usage/troubleshooting docs
- Script features implemented:
  - `--platform <windows|macos|all>` flag (default: all)
  - `--binary-dir <path>` for pre-built binaries
  - `--release <tag>` for downloading from GitHub releases
  - `--dry-run` mode that validates prerequisites and prints execution plan
  - `--boot-timeout`, `--site-dir`, `--help` flags
  - Prerequisite validation (Docker, KVM, images, site directory, binaries)
  - Container lifecycle management with cleanup trap (EXIT/INT/TERM)
  - Linux baseline generation using native binary
  - Windows testing via dockurr/windows with shared folder mechanism
  - macOS testing via dockurr/macos with shared folder mechanism
  - CRLF normalization for Windows output comparison
  - File tree and content comparison against Linux baseline
  - Color-coded status output with clear phase markers
  - Per-platform pass/fail summary report
  - Exit code 0 on success, non-zero on failure
- Dry-run tested successfully: validates Docker, KVM, images, prints execution plan
- Error paths tested: missing binary dir, empty binary dir, missing binaries
- Existing test suite: 16 passed, 0 failed
- Clippy: clean, no warnings
- Fmt: clean
- Files created: `scripts/e2e-cross-platform.sh`, `docs/cross-platform-testing.md`
