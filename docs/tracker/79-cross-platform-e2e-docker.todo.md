# Issue 79: Cross-platform end-to-end testing via Docker (Windows + macOS)

## Problem

We can only test rustkyll on Linux locally. The Unicode panic (#78) was only caught by a user running on Windows. We need automated cross-platform testing.

## Resources

- https://github.com/dockur/windows — Windows in Docker (KVM-based, runs real Windows)
- https://github.com/dockur/macos — macOS in Docker (KVM-based, runs real macOS)

These allow running actual Windows/macOS VMs inside Docker containers on a Linux host.

## Goal

Set up end-to-end testing that builds the DTC site with rustkyll on Windows and macOS, verifying:
1. The binary runs without panics
2. The site builds successfully with correct page count
3. Output matches Linux output (same HTML files generated)

## Approach

1. Use dockur/windows to spin up a Windows container
2. Copy the rustkyll Windows binary and DTC site into the container
3. Run `rustkyll build` and verify output
4. Repeat with dockur/macos for macOS binary
5. Compare outputs across all 3 platforms — they should be identical

This could be:
- A local script for on-demand testing
- A CI job (if the Docker images work in GitHub Actions — they need KVM)
- A scheduled nightly test

## Dependencies

- Issue 78 (Unicode panic fix) should be done first
- Requires KVM support on the host

## Acceptance criteria

- Script exists to run rustkyll on Windows via dockur/windows Docker
- Script exists to run rustkyll on macOS via dockur/macos Docker
- DTC site builds successfully on both Windows and macOS
- Page count matches Linux output
- No panics on any platform
- Output file trees are identical across all 3 platforms
- Results documented
- Instructions for running the tests locally
