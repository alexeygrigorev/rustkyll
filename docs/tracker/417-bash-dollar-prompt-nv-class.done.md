# Issue 417: Bash `$` prompt should get class `nv`

## Problem

In bash code blocks, the `$` at the start of command lines (shell prompt)
should get class `nv` (name.variable) per Jekyll/Rouge, but rustkyll
leaves it as bare text.

Jekyll: `<span class="nv">$ </span>docker build`
Rustkyll: `$ docker build` (bare text)

## Scope

Postprocessing in `src/syntax.rs`: wrap `$ ` at the start of lines
in bash code blocks with `<span class="nv">$ </span>`.

Only match `$ ` (dollar-space) at line start, not `$VAR` or `${VAR}`.

## Baseline

DTC DOM: 789/790, 114 total diffs — neither must worsen.

## Log

### [SWE] 2026-03-28
- Root cause: `postprocess_bash_prompt_lines` only matched `$ ` at absolute line start via `strip_prefix("$ ")`, missing lines with leading whitespace like ` $ docker build`
- TDD: wrote 6 tests (2 failing: leading-space prompt cases)
- Ran tests: 2 FAIL as expected (leading-space prompt not wrapped)
- Fix: changed `postprocess_bash_prompt_lines` to `trim_start()` the line first, then check for `$ ` prefix, preserving the leading whitespace in output; skip if already in a `<span>` to avoid double-wrapping
- Ran tests: all 6 PASS
- Full suite: 2991 lib tests pass, 0 fail; clippy clean; fmt clean
- DOM verification: 789/790 pages match, 103 total diffs (improved from 114 baseline)
- Files modified: `src/syntax.rs`
