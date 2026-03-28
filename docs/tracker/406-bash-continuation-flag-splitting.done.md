# Issue 406: Bash flags after line continuation not individually wrapped

## Problem

In multi-line docker commands, flags on continuation lines (after `\\`)
are not individually wrapped in `<span class="nt">`. Two patterns need fixing:

### Pattern 1: Bare flags on continuation lines

Syntect does not assign the `nt` (name.tag) scope to flags that appear
on continuation lines after `\\\n`. They end up as bare text.

Jekyll:
```html
  <span class="nt">--rm</span> <span class="nt">--name</span> postgresql <span class="se">\\</span>
  <span class="nt">-e</span> <span class="nv">POSTGRES_USER</span><span class="o">=</span><span class="s2">"root"</span> <span class="se">\\</span>
  <span class="nt">-v</span> postgres_volume_local:/var/lib/postgresql/data <span class="se">\\</span>
  <span class="nt">-p</span> 5432:5432 <span class="se">\\</span>
```

Rustkyll (current):
```html
  --rm <span class="nt">--name</span> postgresql <span class="se">\\</span>
  -e POSTGRES_USER=<span class="s2">"root"</span> <span class="se">\\</span>
  -v postgres_volume_local:/var/lib/postgresql/data <span class="se">\\</span>
  -p 5432:5432 <span class="se">\\</span>
```

Bare `-e`, `-v`, `-p`, `--rm`, `--name` must be wrapped in `<span class="nt">`.

### Pattern 2: Merged flag spans

When two flags appear on the same line (e.g. `-it --rm`), syntect emits
them in a single `nt` span. Jekyll emits separate spans per flag.

Jekyll:
```html
docker run <span class="nt">-it</span> <span class="nt">--rm</span> <span class="se">\\</span>
```

Rustkyll (current):
```html
docker run <span class="nt">-it --rm</span> <span class="se">\\</span>
```

## Scope

Post-processing in `src/syntax.rs` within the existing bash post-processing
block (around line 445). Two transformations:

1. **Split merged `nt` spans**: If a `<span class="nt">` contains multiple
   space-separated flags (e.g. `-it --rm`), split into separate spans
   (`<span class="nt">-it</span> <span class="nt">--rm</span>`).

2. **Wrap bare flags on continuation lines**: After a `<span class="se">\\</span>\n`
   sequence, scan for bare flags (`-x` or `--word`) in the text and wrap them
   in `<span class="nt">`.

### Out of scope

- Variable assignment highlighting (`POSTGRES_USER=`) -- handled by issue 404
- `--network` class reclassification (`n` -> `nt`) -- separate concern, the
  `--network=value` pattern is tokenized differently by syntect
- YAML quote entity differences (`"` vs `&quot;`)

### Affected page

- `blog/how-to-run-postgresql-and-pgadmin-with-docker.html` -- the 3 docker
  run command blocks (lines 98-105, 119-128, 140-146 in the source markdown)

### Dependencies

None.

## DTC DOM Baseline

- Page count: 788/790 -- must not drop below this
- Total diff count: 274 -- must not increase above this
- Target: reduce diffs on the postgresql page (currently 129 diffs)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] Merged `nt` spans containing multiple flags are split into one span per flag
- [ ] Bare flags (`-x`, `--word`) on continuation lines after `\\` are wrapped in `<span class="nt">`
- [ ] DTC DOM page match count >= 788/790 (no regression)
- [ ] DTC DOM total diff count <= 274 (no regression)
- [ ] The fix is generic (works for any bash code block with `\\` continuations, not hardcoded to docker commands)
- [ ] All new and existing tests pass

## Test Scenarios

### Unit: Split merged nt flag spans

Input:
```html
<span class="nt">-it --rm</span>
```
Expected:
```html
<span class="nt">-it</span> <span class="nt">--rm</span>
```

Input:
```html
<span class="nt">-e -v -p</span>
```
Expected:
```html
<span class="nt">-e</span> <span class="nt">-v</span> <span class="nt">-p</span>
```

Input (single flag, no split needed):
```html
<span class="nt">--name</span>
```
Expected (unchanged):
```html
<span class="nt">--name</span>
```

### Unit: Wrap bare flags after continuation

Input:
```html
docker run <span class="nt">-it</span> <span class="se">\\</span>
  --rm <span class="nt">--name</span> postgresql <span class="se">\\</span>
  -e POSTGRES_USER <span class="se">\\</span>
  -v path:/path <span class="se">\\</span>
  -p 5432:5432 <span class="se">\\</span>
```
Expected:
```html
docker run <span class="nt">-it</span> <span class="se">\\</span>
  <span class="nt">--rm</span> <span class="nt">--name</span> postgresql <span class="se">\\</span>
  <span class="nt">-e</span> POSTGRES_USER <span class="se">\\</span>
  <span class="nt">-v</span> path:/path <span class="se">\\</span>
  <span class="nt">-p</span> 5432:5432 <span class="se">\\</span>
```

### Unit: Do not wrap non-flag bare words

Input:
```html
<span class="se">\\</span>
  postgresql --name foo <span class="se">\\</span>
```
Expected (only `--name` is a flag, `postgresql` is not):
```html
<span class="se">\\</span>
  postgresql <span class="nt">--name</span> foo <span class="se">\\</span>
```

### Unit: Do not double-wrap already-wrapped flags

Input:
```html
<span class="se">\\</span>
  <span class="nt">--name</span> postgresql <span class="se">\\</span>
```
Expected (unchanged):
```html
<span class="se">\\</span>
  <span class="nt">--name</span> postgresql <span class="se">\\</span>
```

### Integration: DTC DOM baseline

- Build the DTC site and run DOM comparison
- Verify page match count >= 788/790
- Verify total diff count <= 274
- Verify the postgresql page diff count decreases

## Log

### [SWE] 2026-03-28
- TDD: Wrote 7 failing tests first (split merged nt spans, bare flags after continuation, no double-wrap, no wrap non-flags)
- Ran tests: 4 FAILED as expected (split_merged_nt_spans_two_flags, split_merged_nt_spans_three_flags, bare_flags_after_escaped_backslash_continuation, bare_flags_after_single_backslash_continuation)
- Implemented two post-processing passes in src/syntax.rs:
  1. `postprocess_bash_split_merged_nt_flags` -- splits `<span class="nt">-it --rm</span>` into separate spans
  2. `postprocess_bash_wrap_bare_flags_after_continuation` -- wraps bare flags after `<span class="se">\\</span>\n`
- Ran tests: all 7 issue 406 tests PASS
- Full test suite: 2949 lib tests pass, 0 fail; all integration tests pass
- Clippy: clean (no warnings with -D warnings)
- Format: clean (cargo fmt --check passes)
- DTC DOM baseline: 788/790 pages (no regression), 265 total diffs (down from 274, improvement of 9)
- PostgreSQL page: 120 diffs (down from 129, improvement of 9)
- Files modified: src/syntax.rs
