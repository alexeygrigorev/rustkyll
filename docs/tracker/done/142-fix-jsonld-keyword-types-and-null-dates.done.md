# Issue 142: Fix JSON-LD keyword type coercion and null date handling

## Problem

Two minor JSON-LD issues:

1. **String-vs-number in keywords**: When a keyword is a pure number (e.g., `2024`), Jekyll keeps it as a string (`"2024"`) in the JSON-LD keywords array, but rustkyll outputs it as a number (`2024`). 5 instances across 2 files.

2. **Null vs empty string for dates**: When a page has no date, Jekyll outputs `"datePublished": null` but rustkyll outputs `"datePublished": ""`. 2 instances in 1 file (slack/guidelines.html).

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Keywords in JSON-LD are always strings, even if they look like numbers
- Missing dates render as `null` not `""`
- No regressions

## Log

### [SWE] 2026-03-16
- Root cause (keywords): `liquid_to_json()` in jsonify filter called `to_integer()` on string scalars, which succeeds for numeric-looking strings like "2024". The Liquid ScalarCow has distinct `Integer` and `Str` variants internally.
- Fix (keywords): Replaced manual type-checking logic with `serde_json::to_value(s)` which uses ScalarCow's `#[serde(transparent)]` derive to preserve original types. `Integer(2024)` serializes as JSON `2024`, `Str("2024")` serializes as JSON `"2024"`.
- Root cause (dates): `date_to_xmlschema` filter called `to_kstr()` on nil input, which returns empty string, then returned `Value::scalar("")` instead of `Value::Nil`.
- Fix (dates): Added nil check at the start of `date_to_xmlschema` filter: `if input.is_nil() { return Ok(Value::Nil); }`. This ensures `nil | date_to_xmlschema | jsonify` produces `null`.
- Tests added: 4 new tests (3 in jsonify, 1 in date_to_xmlschema)
- Build: 1257 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: src/template/filters/jsonify.rs, src/template/filters/date_to_xmlschema.rs
