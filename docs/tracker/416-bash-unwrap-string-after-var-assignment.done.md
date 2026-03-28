# Issue 416: Bash unwrap string spans after VAR= assignment

## Problem

In bash code blocks, values after `VAR=` assignments are wrapped in
`<span class="s">value</span>` but Jekyll/Rouge leaves them as bare text.

Jekyll: `<span class="nv">DOCKER_IMAGE</span><span class="o">=</span>serverless-ml`
Rustkyll: `<span class="nv">DOCKER_IMAGE</span><span class="o">=</span><span class="s">serverless-ml</span>`

The `<span class="s">` wrapping on unquoted values is wrong.

## Scope

Postprocessing in `src/syntax.rs`: after `<span class="o">=</span>`,
unwrap `<span class="s">VALUE</span>` to bare text when the value is
a single unquoted word (no spaces, no quotes).

Do NOT unwrap quoted strings like `<span class="s2">"value"</span>`.

## Baseline

DTC DOM: 789/790, 122 total diffs — neither must worsen.

## Log

### [SWE] 2026-03-28
- TDD: wrote 7 failing tests first (test_issue416_bash_var_assignment_*)
- Ran tests: FAILS as expected (function not found)
- Implemented `postprocess_bash_var_eq_unwrap_s` in src/syntax.rs
- Added to bash postprocessing pipeline (after line_continuation_se)
- Ran tests: all 7 PASS
- Full suite: 2985+ tests pass, 0 fail
- Clippy: clean, fmt: clean
- DOM verification: 789/790 pages, 114 total diffs (improved from 122 baseline)
- Files modified: src/syntax.rs
- Files renamed: docs/tracker/416-bash-unwrap-string-after-var-assignment.todo.md -> .in-progress.md
