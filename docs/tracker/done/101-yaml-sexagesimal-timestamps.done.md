# Issue 101: Fix YAML sexagesimal podcast timestamp formatting (D13)

Descoped from issue #90. YAML 1.1 interprets `0:30` as sexagesimal (30 seconds) while YAML 1.2 treats it as string. Affects 2 podcast timestamps per episode. High risk to change YAML parser globally.

## Acceptance criteria
- Podcast timestamps `0:30` render as `0:30` not `30` or `0.5`
- No regressions on other YAML parsing

## Log

### [SWE] 2026-03-15

**Investigation findings:**

The feared sexagesimal issue does NOT exist in rustkyll. Both YAML parsers used by the project handle colon-separated timestamps correctly:

1. **`yaml_rust2` (via `Yaml::from_str`)** -- Used by our lenient YAML parser in `src/yaml.rs`. Does NOT implement YAML 1.1 sexagesimal parsing. Values like `0:30` fail integer and float parsing and become `Yaml::String("0:30")`.

2. **`serde_yaml` 0.9** -- Also keeps `0:30` as `String("0:30")`, not converting to integer 30.

3. **Liquid conversion** -- `yaml_to_liquid` in `src/template/context.rs` maps `String("0:30")` to `LiquidValue::scalar("0:30")`, which renders correctly in templates.

Verified against real podcast data: files like `_podcast/datatalksclub-building-scaling-data-community.md` contain unquoted `time: 0:30` which correctly stays as string `"0:30"` through the full pipeline.

**What was done:**
- Added 6 regression tests to document and verify the behavior:
  - 5 tests in `src/yaml.rs` covering `0:30`, `0:00`, `1:30:00`, realistic podcast transcript arrays, and quoted timestamps
  - 1 test in `src/template/context.rs` verifying timestamps survive the YAML-to-Liquid conversion pipeline
- No code changes needed -- existing behavior already meets acceptance criteria

**Test results:** All 1121+ tests pass, 0 fail. Clippy clean, fmt clean.

**Files modified:**
- `src/yaml.rs` -- added 5 sexagesimal regression tests
- `src/template/context.rs` -- added 1 YAML-to-Liquid timestamp conversion test
- `docs/tracker/101-yaml-sexagesimal-timestamps.in-progress.md` -- this log
