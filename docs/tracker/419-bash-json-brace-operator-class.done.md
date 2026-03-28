# Issue 419: Bash JSON braces `{` `}` should get class `o` (operator)

## Problem

In bash code blocks containing inline JSON (like AWS Lambda responses),
braces `{` and `}` should get class `o` (operator) per Jekyll/Rouge.
Syntect doesn't wrap them at all (bare text).

Jekyll: `<span class="o">{</span>`
Rustkyll: `{` (bare text)

## Scope

Postprocessing in `src/syntax.rs`: wrap bare `{` and `}` in bash code
blocks with `<span class="o">`. Only wrap when they appear as JSON
delimiters (not inside strings or other spans).

## Baseline

DTC DOM: 789/790, 95 total diffs — neither must worsen.

## Log

### [SWE] 2026-03-28
- Investigated Jekyll vs rustkyll output for ml-deployment-lambda.html bash blocks
- Jekyll wraps bare `{` and `}` as `<span class="o">` (operator), rustkyll leaves them as bare text
- Wrote 5 TDD tests: json braces wrapped, simple json, no double-wrap, skip dollar-brace, no braces
- Tests failed as expected (function not yet implemented)
- Implemented `postprocess_bash_json_braces()` in src/syntax.rs
  - Walks HTML tracking span depth; only wraps braces at depth 0 (bare text)
  - Skips braces inside existing span content, HTML tags, or `${VAR}` patterns
- Added to bash postprocessor chain in `highlight()` function
- All 5 unit tests pass, full test suite passes (3001 tests, 0 failures)
- Clippy clean, fmt clean
- DOM verification: 789/790 pages, 81 total diffs (improved from 95 baseline by 14 diffs)
- Files modified: src/syntax.rs
