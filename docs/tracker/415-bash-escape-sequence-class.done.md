# Issue 415: Bash escape sequences get wrong class (missing `se`)

## Problem

In bash code blocks, escape sequences like `\\`, `\"` should get class
`se` (string.escape) but syntect produces `p` (punctuation) or no class.

~9-11 diffs from this pattern.

## Scope

Postprocessing in `src/syntax.rs` to remap escaped characters in bash
strings to class `se`.

## Baseline

DTC DOM: 789/790, 132 total diffs — neither must worsen.

## Log

### [SWE] 2026-03-28
- Analyzed Jekyll vs rustkyll output: line continuation `\` + newline had class `p` but Jekyll uses `se`
- Added `postprocess_bash_line_continuation_se()` to remap `<span class="p">\<newline></span>` to `<span class="se">\</span><newline>`
- Updated existing test `test_regression_bash_line_continuation_is_p` -> `test_regression_bash_line_continuation_is_se`
- Tests: 2 new tests (integration + unit) plus updated regression test
- All 2978 tests pass, clippy clean, fmt clean
- DOM: 789/790 pages, 122 total diffs (down from 132)
- Files modified: src/syntax.rs
