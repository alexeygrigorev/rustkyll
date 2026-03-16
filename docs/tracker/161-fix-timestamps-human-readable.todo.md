# Issue 161: Display timestamps as 0:36 not 36.0

## Problem

Issue #155 made timestamps match Jekyll's `36.0` output. But the user wants human-readable `0:36` format. This is a deliberate improvement over Jekyll — timestamps should display as time, not floats.

## Goal

YAML sexagesimal values like `0:36` should display as `0:36` (M:SS format), not `36.0`. This means we intentionally differ from Jekyll here — it's a known, acceptable difference.

Mark this in the DOM comparison as an expected/acceptable diff so it doesn't count as a failure.

## Acceptance criteria

- `0:36` in YAML renders as `0:36` in templates (not `36.0`)
- `1:05:30` renders as `1:05:30` (not `3930.0`)
- DOM comparison tool has an allowlist for timestamp format differences
- All existing tests pass
