# Issue 413: Bash variable substitution ${VAR} gets wrong class

## Problem

In bash code blocks on ml-deployment-lambda, `${VAR}` substitutions have
class mismatches. Syntect produces `k` (keyword) or `p` (punctuation) for
`${`, `}`, and the variable name, but Jekyll/Rouge uses different classes.

Examples:
- `${AWS_ACCOUNT}` — variable name should be `nv`, braces should be bare or `p`
- `[` and `]` in conditionals get wrong classes

~26-31 diffs from this pattern.

## Scope

Postprocessing in `src/syntax.rs` for `${VAR}` patterns in bash blocks.

## Baseline

DTC DOM: 789/790, 132 total diffs — neither must worsen.

## Log

### [SWE] 2026-03-28
- Analyzed Jekyll vs rustkyll bash blocks for ml-deployment-lambda page
- Jekyll: `<span class="k">${</span><span class="nv">VAR</span><span class="k">}</span>`
- Rustkyll before fix: `<span class="p">${</span><span class="n">VAR</span><span class="p">}</span>`
- Added `postprocess_bash_var_substitution()` to remap `p` -> `k` for `${` and `}` braces
- Tests: test_issue413_bash_var_substitution_braces, test_issue413_bash_var_substitution_unit
- All tests pass (2978 unit + integration tests)
- DOM: 789/790 pages, 122 total diffs (down from 132) -- no regression
- Files modified: src/syntax.rs
