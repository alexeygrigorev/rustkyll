# Issue 66: Publish all platform wheels to PyPI (v0.1.1)

## Problem

Only the Linux amd64 wheel was published to PyPI (manually as v0.1.0). Users on macOS, Windows, and Linux ARM64 get "no wheels with a matching platform tag" errors when running `uvx rustkyll`.

Also, Windows ARM64 was requested but never added to the build matrix (descoped from #58, tracked in #65).

## Goal

1. Add Windows ARM64 to the release workflow and wheel builder (absorbs issue #65)
2. Bump version to 0.1.1 (since 0.1.0 is already on PyPI with Linux-only)
3. Push a v0.1.1 tag to trigger the release workflow
4. Verify all 6 platform wheels are published to PyPI

## Approach

1. Add `windows-arm64` target to `.github/workflows/release.yml` build matrix
   - name: `windows-arm64`
   - os: `windows-latest`
   - target: `aarch64-pc-windows-msvc`
   - binary: `rustkyll.exe`
   - asset_name: `rustkyll-windows-arm64.exe`
   - use_cross: `false`
2. Add `windows-arm64` entry to the `release` job's `files:` list in the workflow
3. Add `windows-arm64` target to `scripts/build-wheels.py` TARGETS list
   - asset_name: `rustkyll-windows-arm64.exe`
   - platform_tag: `win_arm64`
   - binary_name: `rustkyll.exe`
4. Update README platform table to include Windows ARM64
5. Bump version to `0.1.1` in all three locations:
   - `Cargo.toml` (line: `version = "0.1.0"`)
   - `python/pyproject.toml` (line: `version = "0.1.0"`)
   - `python/rustkyll/__init__.py` (line: `__version__ = "0.1.0"`)
6. Update existing test assertions that hardcode the target count from 5 to 6:
   - `python/tests/test_build_wheels.py` lines 167 and 193: `self.assertEqual(len(wheels), 5)` must become 6
7. Commit, tag v0.1.1, push tag to trigger the release workflow
8. Monitor the workflow, verify GitHub Release has 6 binaries
9. Verify `uvx rustkyll --help` works on at least Linux after PyPI publish completes

## Dependencies

- Issue 58 (done) -- release workflow exists
- Issue 59 (done) -- wheel builder and PyPI publish job exist
- Absorbs issue 65 (Windows ARM64 build) -- already marked done as absorbed

## Acceptance Criteria

### Code changes (engineer implements locally, before tag push)

- [ ] `.github/workflows/release.yml` build matrix contains 6 entries: linux-amd64, linux-arm64, darwin-amd64, darwin-arm64, windows-amd64, windows-arm64
- [ ] The `windows-arm64` matrix entry uses target `aarch64-pc-windows-msvc`, os `windows-latest`, binary `rustkyll.exe`, asset_name `rustkyll-windows-arm64.exe`, use_cross `false`
- [ ] The `release` job's `files:` list includes `artifacts/rustkyll-windows-arm64.exe/rustkyll-windows-arm64.exe`
- [ ] `scripts/build-wheels.py` TARGETS list contains 6 entries, including `("rustkyll-windows-arm64.exe", "win_arm64", "rustkyll.exe")`
- [ ] `Cargo.toml` version is `"0.1.1"`
- [ ] `python/pyproject.toml` version is `"0.1.1"`
- [ ] `python/rustkyll/__init__.py` has `__version__ = "0.1.1"`
- [ ] All three version strings are identical (`0.1.1`)
- [ ] README platform table has 6 rows: Linux x86_64, Linux ARM64, macOS Intel, macOS Apple Silicon, Windows x86_64, Windows ARM64
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt --check` shows no formatting issues
- [ ] Existing Rust tests pass (`cargo test`)
- [ ] Python wheel-builder tests pass with updated target count (`python -m pytest python/tests/test_build_wheels.py` or `python -m unittest python.tests.test_build_wheels`)

### Post-push verification (after tag push -- done manually or by monitoring CI)

- [ ] v0.1.1 tag exists on GitHub
- [ ] GitHub Actions release workflow completes successfully for all 6 targets
- [ ] GitHub Release page for v0.1.1 lists 6 binary assets
- [ ] All 6 platform wheels are published to PyPI as rustkyll 0.1.1
- [ ] `pip install rustkyll==0.1.1` (or `uvx rustkyll --help`) works on Linux

## Test Scenarios

### Unit: Wheel builder targets (Python)

- Verify `build_wheels.TARGETS` has exactly 6 entries
- Verify `("rustkyll-windows-arm64.exe", "win_arm64", "rustkyll.exe")` is in TARGETS
- Build a dummy wheel for `win_arm64` platform tag, verify wheel filename contains `win_arm64`
- Build a dummy wheel for `win_arm64`, verify `rustkyll/bin/rustkyll.exe` is inside the zip
- Build all 6 dummy wheels from flat directory, verify 6 `.whl` files produced
- Build all 6 dummy wheels from nested directory (GitHub Actions layout), verify 6 `.whl` files produced

### Unit: Version consistency

- Read version from `Cargo.toml`, `python/pyproject.toml`, and `python/rustkyll/__init__.py` -- all three must be `0.1.1`
- `build_wheels.read_version()` returns `"0.1.1"`

### Integration: Release workflow structure (static analysis)

- Parse `.github/workflows/release.yml` and verify the build matrix has 6 `include` entries
- Verify the release job `files:` block lists 6 binary paths
- Verify the `publish-pypi` job still runs `scripts/build-wheels.py`

### Post-deploy: PyPI verification (manual, after CI completes)

- Visit `https://pypi.org/project/rustkyll/0.1.1/#files` and confirm 6 wheels listed
- Run `uvx rustkyll --help` on a Linux machine, confirm it prints help text
- Optionally run `uvx rustkyll --help` on macOS or Windows to confirm cross-platform availability

## Notes

- The Windows ARM64 target (`aarch64-pc-windows-msvc`) can be cross-compiled on `windows-latest` runners by adding the target via `rustup target add`. No `cross` tool needed.
- The `--version` check step in the workflow will work for Windows ARM64 since the runner is x86_64 and cannot execute ARM64 binaries natively. The engineer should either skip the `--version` check for this target or handle it gracefully (the existing `if: "!matrix.use_cross"` condition will run it, but it will fail on ARM64 binary). The engineer must handle this, e.g., by adding a `skip_verify: true` flag to the windows-arm64 matrix entry or adjusting the condition.

## Log

### [PM] 2026-03-14
- Groomed issue from todo to groomed
- Added detailed acceptance criteria split into local code changes and post-push verification
- Added test scenarios for wheel builder, version consistency, workflow structure, and post-deploy
- Noted the --version verification step issue for cross-compiled ARM64 on x86_64 runner
- Updated approach with specific values for matrix entry, TARGETS entry, and version file locations
- Noted that existing test assertions for target count (5) must be updated to 6
- No criteria descoped; all original requirements preserved

### [SWE] 2026-03-14
- Added windows-arm64 matrix entry to `.github/workflows/release.yml` with target `aarch64-pc-windows-msvc`, os `windows-latest`, binary `rustkyll.exe`, asset_name `rustkyll-windows-arm64.exe`, use_cross `false`, skip_verify `true`
- Updated --version verify step condition to skip when `matrix.skip_verify` is true (ARM64 binary cannot run on x86_64 runner)
- Added `rustkyll-windows-arm64.exe` to release job files list
- Added `("rustkyll-windows-arm64.exe", "win_arm64", "rustkyll.exe")` to `scripts/build-wheels.py` TARGETS (now 6 entries)
- Bumped version to 0.1.1 in `Cargo.toml`, `python/pyproject.toml`, `python/rustkyll/__init__.py`
- Added Windows ARM64 row to README platform table (now 6 rows)
- Updated `python/tests/test_build_wheels.py` wheel count assertions from 5 to 6
- Build: compiles without errors
- Clippy: clean (no warnings)
- Fmt: clean
- Rust tests: all pass (11 passed, 0 failed)
- Python tests: all 8 pass (flat dir builds 6 wheels, nested dir builds 6 wheels)
- Files modified: `.github/workflows/release.yml`, `scripts/build-wheels.py`, `Cargo.toml`, `python/pyproject.toml`, `python/rustkyll/__init__.py`, `README.md`, `python/tests/test_build_wheels.py`

### [QA] 2026-03-14
- Rust tests: 11 passed, 0 failed
- Clippy: clean (no warnings)
- Fmt: clean
- Python tests: 25 passed, 0 failed (includes wheel builder tests building 6 wheels each)
- Version consistency: all three files (Cargo.toml, pyproject.toml, __init__.py) report 0.1.1
- AC 1: release.yml build matrix has 6 entries -- PASS
- AC 2: windows-arm64 entry has correct target/os/binary/asset_name/use_cross -- PASS
- AC 3: release job files list includes windows-arm64 artifact path -- PASS
- AC 4: build-wheels.py TARGETS has 6 entries including win_arm64 tuple -- PASS
- AC 5-8: versions all 0.1.1 across Cargo.toml, pyproject.toml, __init__.py -- PASS
- AC 9: README platform table has 6 rows (Linux x86_64, Linux ARM64, macOS Intel, macOS Apple Silicon, Windows x86_64, Windows ARM64) -- PASS
- AC 10: cargo build compiles -- PASS
- AC 11: clippy clean -- PASS
- AC 12: fmt clean -- PASS
- AC 13: Rust tests pass -- PASS
- AC 14: Python tests pass with updated count (6 wheels) -- PASS
- Note: diff includes unrelated changes (ci.yml integration job, compare-output.sh validate-only mode, deleted todo files for issues 68/76) from other work -- not in scope for this issue but does not affect correctness
- VERDICT: PASS

### [PM] 2026-03-14 -- Acceptance Review
- Reviewed full diff and verified each acceptance criterion against the actual file contents
- AC 1 (6 matrix entries in release.yml): VERIFIED -- lines 38-79 of release.yml
- AC 2 (windows-arm64 entry fields): VERIFIED -- target, os, binary, asset_name, use_cross, skip_verify all correct
- AC 3 (release files list): VERIFIED -- line 150 of release.yml
- AC 4 (build-wheels.py 6 TARGETS): VERIFIED -- lines 31-62
- AC 5-8 (version 0.1.1 in all three files): VERIFIED -- Cargo.toml, pyproject.toml, __init__.py all read 0.1.1
- AC 9 (README 6-row platform table): VERIFIED -- lines 37-42 of README.md
- AC 10-14 (build, clippy, fmt, tests): confirmed by QA report
- Post-push verification criteria (tag, CI, PyPI) correctly deferred to after commit
- Noted unrelated changes in working tree (ci.yml integration job, compare-output.sh, deleted todo files, date_to_rfc822 filter) -- not part of issue 66, does not affect correctness
- No criteria descoped
- VERDICT: ACCEPT

### [SWE] 2026-03-14 -- Post-push verification

#### Attempt 1: v0.1.1 tag on commit 1465fc0
- Release workflow run 23095703730 triggered
- `Build darwin-amd64` FAILED: `macos-13` runner is no longer supported by GitHub Actions ("The configuration 'macos-13-us-default' is not supported")
- Other builds were cancelled due to `fail-fast: true`
- Fix: changed `darwin-amd64` os from `macos-13` to `macos-latest` (ARM64 runner, cross-compiles for x86_64); added `skip_verify: true` since ARM64 runner cannot execute x86_64 binary
- Committed fix, deleted old v0.1.1 tag, re-tagged on new commit 7d9cedf

#### Attempt 2: v0.1.1 tag on commit 7d9cedf
- Release workflow run 23095757638 triggered
- All 6 builds PASSED: linux-amd64, linux-arm64, darwin-amd64, darwin-arm64, windows-amd64, windows-arm64
- GitHub Release created with 6 binary assets -- VERIFIED
- `Publish to PyPI` FAILED: trusted publisher not configured on PyPI
  - Error: `invalid-publisher: valid token, but no corresponding publisher`
  - The OIDC token was valid but PyPI has no matching trusted publisher for this repo/workflow
  - The `environment` claim is `MISSING` (workflow does not specify an environment)
  - v0.1.0 was published manually, so trusted publishing was never set up

#### Action needed from repo owner
The repo owner must configure a trusted publisher on PyPI:
1. Go to https://pypi.org/manage/project/rustkyll/settings/publishing/
2. Add a new publisher:
   - PyPI project name: `rustkyll`
   - Owner: `alexeygrigorev`
   - Repository name: `rustkyll`
   - Workflow name: `release.yml`
   - Environment name: (leave blank)
3. After configuring, re-run the failed `Publish to PyPI` job:
   `gh run rerun 23095757638 --repo alexeygrigorev/rustkyll --failed`

#### Verification checklist
- [x] v0.1.1 tag exists on GitHub
- [x] GitHub Actions release workflow: 6/6 builds passed
- [x] GitHub Release page for v0.1.1 lists 6 binary assets
- [ ] Trusted publisher configured on PyPI (BLOCKED -- needs repo owner)
- [ ] All 6 platform wheels published to PyPI as rustkyll 0.1.1 (BLOCKED)
- [ ] `uvx rustkyll --help` works on Linux (BLOCKED)

## Status

v0.1.4 released and published to PyPI with all 6 platform wheels. GitHub Release has all 6 binaries. User confirmed `uvx rustkyll -V` works on Windows (v0.1.3, then v0.1.4).

### USER ACTION REQUIRED

- [ ] User to verify `uvx rustkyll build` works on Windows with v0.1.4 (Unicode panic fixed)
- [ ] User to verify `uvx rustkyll serve` works on Windows with v0.1.4

Once confirmed, this issue can be moved to done.
