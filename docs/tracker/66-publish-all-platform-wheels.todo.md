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

1. Add `windows-arm64` target to `.github/workflows/release.yml` matrix
2. Add `windows-arm64` target to `scripts/build-wheels.py` TARGETS list
3. Update README platform table to include Windows ARM64
4. Bump version in Cargo.toml, python/pyproject.toml, and python/rustkyll/__init__.py to 0.1.1
5. Commit, tag v0.1.1, push tag to trigger the release workflow
6. Monitor the workflow, verify GitHub Release has 6 binaries
7. Verify `uvx rustkyll --help` works on at least Linux after PyPI publish completes

## Dependencies

- Issue 58 (done) — release workflow exists
- Issue 59 (done) — wheel builder and PyPI publish job exist
- Absorbs issue 65 (Windows ARM64 build)

## Acceptance criteria

- Windows ARM64 target added to release workflow and wheel builder
- Version bumped to 0.1.1 in Cargo.toml, pyproject.toml, and __init__.py
- v0.1.1 tag pushed to GitHub
- GitHub Actions release workflow completes successfully for all 6 targets
- GitHub Release created with 6 binaries: linux-amd64, linux-arm64, darwin-amd64, darwin-arm64, windows-amd64, windows-arm64
- All 6 platform wheels published to PyPI as rustkyll 0.1.1
- `uvx rustkyll --help` works on Linux
- `uvx rustkyll --help` works on Windows (if testable)
- README platform table includes Windows ARM64
