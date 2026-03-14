# Issue 80: Fix CI failure — empty tools pages

## Problem

CI integration test `test_dtc_output_no_empty_html_files` fails because `tools/modelstore.html` and `tools/obsei.html` are 0 bytes. These are tools collection items with no layout and empty markdown body — they produce 0-byte output, which matches Jekyll's behavior (Jekyll also produces empty/near-empty files for these).

## Goal

Fix the CI test so it doesn't fail on legitimately empty pages that match Jekyll's output.

## Approach

Either:
1. Exclude known empty pages from the "no empty files" check (allowlist)
2. Change the threshold from 0 bytes to match Jekyll's behavior (Jekyll produces 1 byte — a newline)
3. Make tools collection items output at least a newline like Jekyll does

## Dependencies

None

## Acceptance criteria

- CI integration test passes
- The fix correctly handles legitimately empty collection items
- No regressions on other tests
