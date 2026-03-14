# Issue 65: Add Windows ARM64 binary build

## Problem

Issue #58 only included 5 targets but Windows ARM64 was requested and descoped without creating a follow-up issue.

## Goal

Add Windows ARM64 (aarch64) to the release workflow matrix.

## Approach

1. Add `windows-arm64` target to .github/workflows/release.yml matrix
2. Use cross-compilation from windows-latest or a dedicated approach
3. Binary name: rustkyll-windows-arm64.exe
4. Update scripts/build-wheels.py to include the new target
5. Update README platform table

## Dependencies

- Issue 58 (done)

## Acceptance criteria

- Release workflow includes windows-arm64 target
- Binary is named rustkyll-windows-arm64.exe
- Wheel builder includes the new platform
- README platform table updated
- All existing targets still build correctly
