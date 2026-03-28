# Issue 421: Bash brackets `[` `]` and pipe `|` operator classes

## Problem

In bash code blocks, `[`, `]`, and `|` operators have wrong classes.
Jekyll/Rouge uses `o` (operator) for these, but syntect uses `k`
(keyword) for `[`/`]` and `ow` (operator.word) for `|`.

Also: bare `]` after text like `run|exec]` is not wrapped at all.

## Scope

Postprocessing in `src/syntax.rs`:
1. Remap `<span class="k">[</span>` and `<span class="k">]</span>` to class `o`
2. Remap `<span class="ow">|</span>` to class `o`

## Baseline

DTC DOM: 789/790, 70 total diffs — neither must worsen.

## Log

### [SWE] 2026-03-28
- Investigated actual Jekyll vs rustkyll output for `docker [run|exec] ${DOCKER_IMAGE}`:
  - Jekyll: `<span class="o">[</span>run|exec]` (bracket is `o`, `]` and `|` are bare text)
  - Rustkyll: `<span class="k">[</span>run|exec<span class="k">]</span>` (bracket is `k`, `]` wrapped)
- Also for `aws ecr get-login-password | \`:
  - Jekyll: pipe `|` is bare text
  - Rustkyll: `<span class="ow">|</span>` (wrapped as operator-word)
- TDD: wrote 3 tests (bracket remap, pipe remap, variable expansion preserved) -- all FAILED as expected
- Implemented `postprocess_bash_bracket_and_pipe()` in src/syntax.rs:
  - `<span class="k">[</span>` -> `<span class="o">[</span>`
  - `<span class="k">]</span>` -> bare `]`
  - `<span class="ow">|</span>` -> bare `|`
- Ran tests: all 3 PASS
- Full test suite: all pass (3007+ tests), clippy clean, fmt clean
- DOM verification: DTC 789/790 pages, 3 total diffs on the 1 mismatched page -- no regression
- Files modified: src/syntax.rs
