# Issue 155: Fix timestamp formatting (209 diffs)

## Problem

Podcast timestamps like `36.0` should render as `0:36`. YAML values like `0:36` are parsed as sexagesimal (36 seconds = 36.0) by some parsers. Need to format these back as `M:SS` or `H:MM:SS`.

## Acceptance criteria

- Podcast timestamps render as `M:SS` format matching Jekyll
- 209 DOM diffs eliminated
- TDD: failing test, fix, test passes

## Log

### [SWE] 2026-03-16

**Root cause analysis:**

The DOM diff report showed 209 diffs where rustkyll output strings like `0:36` but Jekyll
output `36.0`. Investigation revealed:

- Ruby's YAML parser (Psych) implements YAML 1.1 sexagesimal (base-60) parsing, converting
  unquoted `0:36` to float 36.0 (0*60 + 36 = 36)
- Our yaml_rust2-based parser did NOT do sexagesimal, keeping `0:36` as a string
- In podcast YAML, timestamps < 1 minute are unquoted (`time: 0:36`), while timestamps
  >= 1 minute are quoted (`time: '1:05'`) and stay as strings in both parsers
- Issue 101 had documented that our parser correctly preserves timestamps as strings,
  but that was the OPPOSITE of what Jekyll does -- creating the 209 diffs

**Fix applied:**

Added YAML 1.1 sexagesimal parsing to `LenientYamlLoader` in `src/yaml.rs`:
- New `parse_sexagesimal()` function detects colon-separated digit patterns (e.g. `N:NN`, `N:NN:NN`)
- Converts to base-60 float value (e.g. `0:36` -> 36.0)
- Stores result as `Yaml::String("36.0")` so Liquid renders it exactly as Jekyll does
  (with `.0` suffix for whole numbers)
- Only applies to unquoted plain scalars; quoted strings like `'1:05'` remain unchanged
- URLs with colons (e.g. `https://...`) are not affected (contain non-digit characters)

**Tests:**
- Updated 4 existing sexagesimal tests in `src/yaml.rs` to expect new behavior
- Added `test_sexagesimal_parse_function` with 11 cases (valid and invalid inputs)
- Added `test_url_not_parsed_as_sexagesimal` regression test
- Updated context test `test_sexagesimal_timestamp_becomes_float_in_liquid`
- Added `test_sexagesimal_timestamp_renders_as_float_in_template` end-to-end test
- Verified site build: podcast timestamps now match Jekyll exactly

**Build:** 1297+ tests pass, 0 fail. Clippy clean, fmt clean.

**Files modified:**
- `src/yaml.rs` -- added `parse_sexagesimal()`, `format_sexagesimal_float()`, integrated into LenientYamlLoader, updated tests
- `src/template/context.rs` -- updated sexagesimal context test, added rendering test
