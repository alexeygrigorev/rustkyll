# Issue 418: Bash angle bracket placeholders wrapped as HTML

## Problem

In bash code blocks, angle-bracket placeholders like `<path-to-Dockerfile>`
and `<stack-name>` are being parsed by syntect as HTML-like elements,
producing extra spans. Jekyll/Rouge treats them as literal text.

Jekyll: `./<path-to-Dockerfile>` (bare text)
Rustkyll: `./<span class="o">&lt;</span>path-to-Dockerfile<span class="o">&gt;</span>` (or similar)

## Scope

Postprocessing in `src/syntax.rs`: detect angle bracket placeholder patterns
in bash blocks and unwrap/normalize them to match Jekyll output.

## Baseline

DTC DOM: 789/790, 103 total diffs — neither must worsen.

## Log

### [SWE] 2026-03-28
- Investigated Jekyll vs rustkyll output for ml-deployment-lambda.html
- Jekyll: `./&lt;path-to-Dockerfile&gt;` (plain HTML entities)
- Rustkyll: `./<span class="o">&lt;</span>path-to-Dockerfile<span class="o">&gt;</span>` (wrapped in operator spans)
- TDD: Wrote 3 unit tests (unwrap placeholder, stack-name, preserve real operators)
- Tests compile-failed as expected (function not yet defined)
- Implemented `postprocess_bash_angle_bracket_placeholders()` in src/syntax.rs
- Wired into bash postprocessing chain
- Tests: all 3 pass
- Full suite: all pass, clippy clean, fmt clean
- DOM verification: 789/790 pages, 95 total diffs (improved from 103 baseline)
- Files modified: src/syntax.rs
- Files renamed: docs/tracker/418-bash-angle-bracket-placeholder.todo.md -> .in-progress.md
