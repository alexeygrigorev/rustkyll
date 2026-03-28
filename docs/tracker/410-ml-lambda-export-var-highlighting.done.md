# Issue 410: Bash `export VAR=val` highlighting (ml-lambda page)

## Problem

On the ml-deployment-lambda page, `export` statements have wrong span classes.

Source markdown (inside a bash code block):
```
export AWS_REGION=eu-central-1
export AWS_ACCOUNT=PUT_VALUE_HERE
```

Jekyll (Rouge) produces:
```html
<span class="nb">export</span> <span class="nv">AWS_REGION</span><span class="o">=</span>eu-central-1
<span class="nb">export</span> <span class="nv">AWS_ACCOUNT</span><span class="o">=</span>PUT_VALUE_HERE
```

Rustkyll currently produces:
```html
<span class="k">export</span> AWS_REGION<span class="o">=</span>eu-central-1
```

Two problems:
1. `export` gets class `k` (keyword) instead of `nb` (builtin)
2. The `VAR_NAME=` after `export` is not wrapped with `nv`/`o` spans (the existing `postprocess_bash_env_var_assignments` handles bare-text `VAR=` but `export` is inside a `<span class="k">` so the variable name may also end up inside a span or bare depending on syntect output)

## Scope

1. Remap `<span class="k">export</span>` to `<span class="nb">export</span>` in bash post-processing (same pattern as `postprocess_bash_local`)
2. Ensure `VAR_NAME=value` after `export` gets `nv` + `o` wrapping (the existing `postprocess_bash_env_var_assignments` should already handle this once export is remapped, but verify)

Implementation location: `src/syntax.rs`, add a `postprocess_bash_export` function called from the bash post-processing block (lines 445-451), following the same pattern as `postprocess_bash_local`.

## Dependencies

None. Issue 407 (env var assignments) is already implemented.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] Bash `export` is classified as `nb` (builtin), not `k` (keyword)
  - `highlight_code("bash", "export AWS_REGION=eu-central-1\n")` must contain `<span class="nb">export</span>`
  - Must NOT contain `<span class="k">export</span>`
- [ ] Variable name after `export` is wrapped with `nv` class and `=` with `o` class
  - Output must contain `<span class="nv">AWS_REGION</span><span class="o">=</span>`
- [ ] Full line `export AWS_REGION=eu-central-1` produces:
  `<span class="nb">export</span> <span class="nv">AWS_REGION</span><span class="o">=</span>eu-central-1`
- [ ] `export` inside identifiers is NOT remapped (e.g. `exported_var` must not get `nb` class)
- [ ] No double-wrapping: `<span class="nb"><span class="nb">export</span></span>` must not occur
- [ ] DTC DOM baseline: 788/790 pages matched -- must not drop below this
- [ ] DTC DOM total diffs: 169 -- must not increase (should decrease since ml-lambda diffs will be fixed)

## Test Scenarios

### Unit: export remap
- `highlight_code("bash", "export FOO=bar\n")` contains `<span class="nb">export</span>`
- `highlight_code("bash", "export FOO=bar\n")` contains `<span class="nv">FOO</span><span class="o">=</span>`
- `highlight_code("bash", "export AWS_REGION=eu-central-1\nexport AWS_ACCOUNT=PUT_VALUE_HERE\n")` produces correct spans for both lines

### Unit: no false positives
- `highlight_code("bash", "exported_data=1\n")` does NOT contain `<span class="nb">export</span>`
- `highlight_code("bash", "echo export\n")` -- `export` after echo should still be `nb` if syntect classifies it as keyword (this is fine, it is a builtin)

### Unit: no double wrapping
- `highlight_code("bash", "export VAR=1\n")` does NOT contain nested `<span class="nb"><span class="nb">`

### Integration: DOM regression
- Build DTC site, verify 788/790 pages matched, total diffs <= 169

## Baseline

- DTC DOM: 788/790 pages matched, 169 total differences
- These numbers must not worsen after this change

## Log

### [SWE] 2026-03-28
- Wrote 6 failing tests in src/syntax.rs (test_issue410_bash_export_*)
- Ran tests: 4 FAILED as expected -- export classified as `k`, var name as `n`, value wrapped in `s`
- Implemented `postprocess_bash_export()` in src/syntax.rs:
  - Remaps `<span class="k">export</span>` to `<span class="nb">export</span>`
  - Remaps `<span class="n">VAR</span>` to `<span class="nv">VAR</span>` after export
  - Unwraps `<span class="s">value</span>` after `=` in export context
- Added call in bash post-processing block (line 449)
- Ran tests: all 6 PASS
- Full test suite: all pass, 0 failures
- Clippy: clean (no warnings from rustkyll)
- Fmt: clean
- DTC DOM: 788/790 pages matched, 156 total diffs (improved from 169 baseline)
- Files modified: src/syntax.rs
