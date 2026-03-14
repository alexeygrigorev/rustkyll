# Issue 58: Cross-platform binary builds

## Problem

rustkyll currently only builds for the developer's local platform. To be useful as a Jekyll replacement, it needs pre-built binaries for all major platforms.

## Goal

Set up CI/CD (GitHub Actions) to compile and release rustkyll binaries for:

- Linux x86_64 (amd64)
- Linux aarch64 (arm64)
- macOS x86_64 (Intel)
- macOS aarch64 (Apple Silicon)
- Windows x86_64

## Approach

1. Create a GitHub Actions release workflow triggered on git tags (e.g. v0.1.0)
2. Use cross-compilation or matrix builds for each target
3. Package binaries with appropriate naming (e.g. rustkyll-linux-amd64, rustkyll-darwin-arm64)
4. Upload as GitHub Release assets
5. Optionally add a Makefile or script for local cross-compilation

## Dependencies

None

## Acceptance criteria

- GitHub Actions workflow exists for release builds
- Workflow produces binaries for all 5 targets listed above
- Binaries are uploaded as GitHub Release assets
- Each binary runs and prints --help on its target platform (test in CI where possible)
- README updated with installation instructions pointing to releases
