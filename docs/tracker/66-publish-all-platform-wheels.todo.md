# Issue 66: Publish all platform wheels to PyPI

## Problem

Only the Linux amd64 wheel was published to PyPI (manually). Users on macOS, Windows, and Linux ARM64 get "no wheels with a matching platform tag" errors when running `uvx rustkyll`.

## Goal

Trigger the GitHub Actions release workflow to build all 5 platform binaries and publish all 5 platform-tagged wheels to PyPI.

## Approach

1. Create a git tag (e.g. v0.1.0) and push it to trigger the release workflow
2. The workflow will build binaries for all 5 targets, create a GitHub Release, and publish wheels to PyPI
3. Verify `uvx rustkyll --help` works on at least Linux and one other platform after publishing

Note: PyPI version 0.1.0 already exists with only the Linux wheel. We may need to bump to 0.1.1 or delete the existing version first.

## Dependencies

- Issue 58 (done) — release workflow exists
- Issue 59 (done) — wheel builder and PyPI publish job exist

## Acceptance criteria

- All 5 platform wheels published to PyPI (linux-amd64, linux-arm64, darwin-amd64, darwin-arm64, windows-amd64)
- `uvx rustkyll --help` works on Linux
- `uvx rustkyll --help` works on macOS (if testable)
- `uvx rustkyll --help` works on Windows (if testable)
- GitHub Release created with all 5 binaries attached
- Version numbers consistent across Cargo.toml, pyproject.toml, and PyPI
