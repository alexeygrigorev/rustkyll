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

## Log

### [SWE] 2026-03-16
- Reverted sexagesimal-to-float conversion in src/yaml.rs
- Replaced `parse_sexagesimal()` (float conversion) and `format_sexagesimal_float()` with `is_sexagesimal()` (detection only)
- Sexagesimal values like `0:36`, `1:05:30` now kept as original strings instead of being converted to `36.0`, `3930.0`
- Updated 8 tests in src/yaml.rs to expect human-readable format
- Updated 2 tests in src/template/context.rs to expect human-readable format
- Added `is_acceptable_sexagesimal_diff()` and `filter_acceptable_diffs()` to scripts/dom_compare.py
- Integrated filtering into both dom_compare.py and dom_compare_full.py
- Added 9 Python tests for the sexagesimal allowlist in scripts/test_dom_compare.py
- Build: 1524 Rust tests pass (0 fail), clippy clean, fmt clean
- Python: 47 DOM comparison tests pass (0 fail)
- Files modified: src/yaml.rs, src/template/context.rs, scripts/dom_compare.py, scripts/dom_compare_full.py, scripts/test_dom_compare.py
