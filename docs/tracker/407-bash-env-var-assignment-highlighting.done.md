# Issue 407: Bash `-e VAR=val` environment variable highlighting

## Problem

In `docker run -e VAR="val"` patterns, the variable name and `=` are
not highlighted.

Jekyll/Rouge:
```html
<span class="nt">-e</span> <span class="nv">POSTGRES_USER</span><span class="o">=</span><span class="s2">"root"</span>
```

Rustkyll:
```html
-e POSTGRES_USER=<span class="s2">"root"</span>
```

Missing: `-e` flag (`nt`), variable name (`nv`), `=` operator (`o`).

## Scope

Postprocessing in `src/syntax.rs` for the specific `VAR_NAME=` pattern
in bash code blocks. Must wrap:
1. The `-e` flag with `nt` class (when bare, after continuation)
2. The `UPPER_CASE_VAR` name with `nv` class
3. The `=` with `o` class

## Acceptance Criteria

1. Bare `UPPER_CASE_VAR=` (not already inside a `<span>`) in bash code blocks gets
   wrapped as `<span class="nv">VAR</span><span class="o">=</span>`.

   Input (after syntect):
   ```
     -e POSTGRES_USER=<span class="s2">"root"</span>
   ```
   Expected output:
   ```
     <span class="nt">-e</span> <span class="nv">POSTGRES_USER</span><span class="o">=</span><span class="s2">"root"</span>
   ```

2. Bare `-e` flags after line continuation (`<span class="se">\\</span>\n`)
   get wrapped as `<span class="nt">-e</span>`.

3. Variables already inside a `<span>` are NOT double-wrapped.

4. Only `[A-Z][A-Z0-9_]*=` patterns are matched (uppercase env vars).

5. DTC DOM baseline: 788/790 pages, 265 total diffs -- neither must worsen.

## Test Scenarios

- `docker run -e POSTGRES_USER="root"` produces correct nv/o/s2 spans
- Multi-line with continuations wraps each `-e VAR="val"` correctly
- Already-wrapped variables are not touched
- Non-uppercase patterns like `foo=bar` are not wrapped
- Unicode variable values work correctly

## Baseline

DTC DOM: 788/790 pages, 265 total diffs.

## Log

### [SWE] 2026-03-28
- Wrote 8 tests first (TDD): test_issue407_bash_env_var_assignment_nv_o, test_issue407_bash_env_var_with_e_flag, test_issue407_already_wrapped_not_doubled, test_issue407_lowercase_not_matched, test_issue407_unicode_value, test_issue407_bare_e_flag_wrapped, test_issue407_var_inside_span_not_touched, test_issue407_full_highlight_integration
- Ran tests: 5 FAILED as expected (stub returns input unchanged), 2 passed (already-wrapped and lowercase correctly unchanged)
- Implemented `postprocess_bash_env_var_assignments` and helpers in src/syntax.rs
- Hooked into bash postprocessing chain
- Ran tests: all 8 PASS
- Full test suite: 2957+ passed, 0 failed
- Clippy: clean (no rustkyll warnings)
- Fmt: clean
- DTC DOM: 788/790 pages (unchanged), 169 total diffs (improved from 265 baseline)
- Files modified: src/syntax.rs, docs/tracker/407-bash-env-var-assignment-highlighting.in-progress.md
