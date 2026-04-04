# Issue 572: YAML multiline quoted scalar parsing fails for programming-historian

## Problem

Rustkyll fails to parse 20 front matter files from the programming-historian site because their YAML contains multiline quoted scalars with continuation lines that are less indented than the first content line. The YAML spec allows this, and Jekyll/Ruby's Psych YAML parser handles it, but rustkyll's YAML parser rejects it.

### Concrete example

From `websites/programming-historian/en/lessons/transliterating.md`:
```yaml
abstract: "This lesson shows how to use Python to transliterate automatically a
list of words from a language with a non-Latin alphabet to a
standardized format using the American Standard Code for Information
Interchange (ASCII) characters."
```

The continuation lines (`list of words...`, `standardized format...`, `Interchange...`) are at column 1, while the opening quote is at column 11 (after `abstract: "`). The YAML spec allows continuation lines in double-quoted scalars to be at any indentation level, but rustkyll's parser requires them to be indented at or beyond the opening line's level.

### Error message
```
YAML scan/parse error: invalid indentation in quoted scalar at byte 320 line 17 column 11
```

## Affected Sites

- programming-historian: 20 files fail to parse, contributing to only 164/697 pages matching
  - All 20 failures are the same error type: "invalid indentation in quoted scalar"
  - These are lesson pages with `abstract:` or `description:` fields containing long multiline quoted text

## Root Cause

The YAML parser (likely `serde_yaml` or `yaml-rust2`) enforces stricter indentation rules for quoted scalars than the YAML 1.1/1.2 spec requires. In YAML, double-quoted scalars can span multiple lines and continuation lines are folded regardless of indentation. The parser should accept these valid YAML documents.

## Scope

- Fix the YAML front matter parsing to accept multiline quoted scalars with reduced indentation on continuation lines
- Options:
  1. Pre-process the YAML to re-indent continuation lines before parsing
  2. Switch to a more lenient YAML parser
  3. Add a fallback: if strict parse fails with "invalid indentation in quoted scalar", try re-indenting and re-parsing
- Must not break any currently-working YAML parsing

## Baseline

- DTC: 789/790 matched (163 total diffs). Must not regress.
- Programming-historian: 164/697 matched (82813 total diffs). Should improve by at least recovering the 20 failed pages.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new ones
- [ ] The 20 programming-historian files with multiline quoted scalars parse successfully
- [ ] The `abstract` field value is correctly extracted as a single string with spaces (not newlines)
- [ ] No regression in parsing of other sites' YAML front matter
- [ ] DTC DOM match count does not drop below 789/790
- [ ] Programming-historian page count increases (fewer "failed to parse" errors)

## Test Scenarios

### Unit: Multiline quoted scalar parsing
- Parse YAML with `abstract: "line one\nline two\nline three"` where continuation lines are at column 1
- Verify the extracted value is `"line one line two line three"` (folded with spaces)
- Parse YAML with properly indented continuation lines (regression check)
- Parse YAML with multiline quoted scalar in a nested context

### Unit: Front matter extraction
- Parse a full front matter block matching the programming-historian pattern
- Verify all fields including the multiline `abstract` are correctly extracted

### Integration: Programming-historian build
- Build the programming-historian site, verify fewer "failed to parse" errors
- Verify the previously-failing pages now generate HTML output

## Dependencies

None.
