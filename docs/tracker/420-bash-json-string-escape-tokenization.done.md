# Issue 420: Bash JSON string escape tokenization

## Problem

In bash code blocks with inline JSON strings, escaped quotes `\"` and
the JSON content are tokenized differently between Jekyll/Rouge and
rustkyll/syntect. Jekyll breaks `"{\\"prediction\\": \\"1\\"}"` into
multiple spans with `se` (string escape) for each `\"`, while rustkyll
keeps the entire string in one `s2` span.

## Scope

Investigate the exact pattern and implement targeted postprocessing in
`src/syntax.rs` if possible. If the token structure is too different
to fix with postprocessing, report findings and close.

## Baseline

DTC DOM: 789/790, 81 total diffs — neither must worsen.

## Log

### [SWE] 2026-03-28
- Investigated exact HTML difference between Jekyll and rustkyll for `ml-deployment-lambda.html`
- Jekyll splits `<span class="s2">"{\"prediction\": \"1\"}"</span>` into alternating `s2` and `se` spans
- Rustkyll keeps entire string in one `s2` span
- Wrote test `test_issue420_bash_json_string_escape_tokenization`: FAILS as expected (no `se` spans)
- Implemented `postprocess_bash_json_string_escapes()` in `src/syntax.rs` - splits `s2` spans containing `\"` into alternating `s2`/`se` spans
- Hooked into bash postprocessing chain after `postprocess_bash_json_braces`
- Ran test: PASSES
- All tests: 3004 passed, 0 failed
- Clippy clean, fmt clean
- DTC DOM: 789/790 pages, 70 total diffs (improved from 81 baseline - 11 fewer diffs)
- Files modified: `src/syntax.rs`
