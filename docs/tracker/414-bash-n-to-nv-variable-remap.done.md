# Issue 414: Bash variable names get class `n` instead of `nv`

## Problem

In bash code blocks, variable names in certain contexts get class `n`
(generic name) instead of `nv` (name.variable). This happens in:
- Variable assignments: `DOCKER_IMAGE=serverless-ml`
- After `export`: already partly fixed by #410
- In `${VAR}` substitutions

~11-13 diffs from this pattern.

## Scope

Add postprocessing in `src/syntax.rs` to remap `<span class="n">VAR</span>`
to `<span class="nv">VAR</span>` when VAR is an UPPER_CASE name in bash
code blocks. Must not remap lowercase names.

## Baseline

DTC DOM: 789/790, 132 total diffs — neither must worsen.

## Log

### [SWE] 2026-03-28
- Analyzed Jekyll vs rustkyll output: uppercase var names like ECR_REPO, DOCKER_IMAGE had class `n` instead of `nv`
- Added `postprocess_bash_n_to_nv_uppercase()` to remap `<span class="n">UPPER_CASE</span>` to `<span class="nv">...</span>`
- Only matches all-uppercase names with digits/underscores; does NOT remap lowercase or mixed-case
- Tests: 4 tests (uppercase, lowercase, mixed-case, digits)
- All 2978 tests pass, clippy clean, fmt clean
- DOM: 789/790 pages, 122 total diffs (down from 132)
- Files modified: src/syntax.rs
