# Issue 412: Bash `--network`/`--name` flag argument gets wrong class

## Problem

In bash code blocks, arguments to `--network` and `--name` flags get class
`n` (generic name) instead of being bare text. This shifts the DOM alignment.

Jekyll: `<span class="nt">--network</span> pg-network <span class="se">\\</span>`
Rustkyll: `<span class="nt">--network</span> <span class="n">pg-network</span> ...`

The `pg-network` and `pg-database` values should be bare text, not wrapped
in `<span class="n">`.

## Scope

Postprocessing in `src/syntax.rs`: unwrap `<span class="n">` when it
follows a `<span class="nt">--flag</span>` pattern (flag argument values
should be bare text, not name-class spans).

## Baseline

- DTC DOM: 788/790, 156 total diffs — neither must worsen
- postgresql page: 24 diffs — should decrease

## Log

### [SWE] 2026-03-28
- Root cause: syntect classifies `--network` as `n` (name) instead of `nt` (name.tag/flag),
  and wraps the value after `=` in `<span class="s">` instead of leaving it bare.
- Added `postprocess_bash_flag_argument_scope()` in `src/syntax.rs` with two passes:
  1. Remap `<span class="n">--FLAG</span>` to `<span class="nt">--FLAG</span>` for content starting with `--`
  2. Unwrap `<span class="s">VALUE</span>` to bare `VALUE` after `</span><span class="o">=</span>` when preceded by a `nt` flag span
- Wired into bash postprocessing chain after `postprocess_bash_env_var_assignments`
- Tests added: 7 unit tests (n->nt remap, s unwrap, combined, non-flag guard, standalone guard, unicode, full integration)
- All tests pass: 7/7 issue-specific, full suite green
- Clippy clean, fmt clean
- DTC DOM: **789/790 pages, 132 total diffs** (was 788/790, 156 diffs) -- improved both metrics
- postgresql page: **0 diffs** (was 24) -- fully matches Jekyll now
- Files modified: `src/syntax.rs`
