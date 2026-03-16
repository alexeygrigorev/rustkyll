# Issue 118: Add Windows and macOS CI integration tests

## Problem

Integration tests only run on Linux. We need to verify rustkyll builds and runs correctly on Windows and macOS in CI.

## Goal

Add Windows and macOS jobs to the integration workflow (integration.yml). At minimum:
- Build rustkyll on each platform
- Run the unit test suite
- Build the DTC site and verify page count
- Verify no panics (the Unicode panic #78 was Windows-only)

## Approach

GitHub Actions provides windows-latest and macos-latest runners. Add matrix jobs:
1. Build with cargo build --release
2. Run cargo test (unit tests only)
3. Clone DTC site only and run rustkyll build
4. Verify output page count matches Linux

Only DTC site needed for cross-platform — no need to clone all 16 sites on Windows/macOS.

## Dependencies

None

## Acceptance criteria

- integration.yml has Windows and macOS jobs
- cargo build succeeds on all 3 platforms
- cargo test passes on all 3 platforms
- DTC site builds on all 3 platforms with same page count
- No platform-specific panics
- Jobs run nightly alongside Linux integration tests
