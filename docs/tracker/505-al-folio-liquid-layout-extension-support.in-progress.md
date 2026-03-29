# Issue 505: Support .liquid layout file extension for al-folio

## Problem

rustkyll only loads layout files with the `.html` extension (`layout.rs` line 1018: `if !filename.ends_with(".html")`). The al-folio theme uses `.liquid` as the extension for all 13 of its layout files:

- `about.liquid`, `archive.liquid`, `bib.liquid`, `book-review.liquid`, `book-shelf.liquid`, `course.liquid`, `cv.liquid`, `default.liquid`, `distill.liquid`, `none.liquid`, `page.liquid`, `post.liquid`, `profiles.liquid`

Because none of these layouts are loaded, **every single page** in the al-folio build is output as raw rendered content without any HTML document structure (`<html>`, `<head>`, `<body>`, navigation, footer, etc.). This is the single biggest blocker for al-folio -- it accounts for all 42 DOM comparison failures.

Jekyll supports both `.html` and `.liquid` extensions for layouts. The `.liquid` extension is increasingly common in modern themes.

## Root Cause

In `src/template/layout.rs`, the `load_layouts` function (around line 1018) filters out any file that does not end in `.html`:

```rust
if !filename.ends_with(".html") {
    continue;
}
```

And the layout name is derived by stripping `.html` (line 1028):

```rust
.strip_suffix(".html")
```

## Scope

1. Modify `load_layouts` to also accept `.liquid` files as valid layout files.
2. Strip either `.html` or `.liquid` from the filename to derive the layout name.
3. Verify that al-folio layouts are loaded and applied correctly.
4. Ensure no regression on existing sites that use `.html` layouts.

## Baseline

- al-folio DOM: 3/45
- DTC DOM baseline: 790/790

## Acceptance Criteria

- [ ] `load_layouts` loads files with both `.html` and `.liquid` extensions from `_layouts/`.
- [ ] Layout names are derived correctly: `post.liquid` becomes `"post"`, `vendor/compress.html` stays `"vendor/compress"`.
- [ ] Building al-folio produces pages wrapped in `<html>`, `<head>`, `<body>` tags (the `default.liquid` layout is applied).
- [ ] The al-folio DOM match count improves from 3/45 (record exact new count in log).
- [ ] DTC DOM match count does not drop below 790/790.
- [ ] `cargo build` compiles without errors; `cargo clippy` clean; `cargo fmt` clean.
- [ ] Existing sites that use `.html` layouts continue to work (no regression).

## Test Scenarios

### Unit: layout loading with .liquid extension
- Create a temp `_layouts/` directory with both `default.html` and `post.liquid` files, call `load_layouts`, verify both are loaded with correct names ("default" and "post").
- Create a layout named `page.liquid` with front matter `layout: default`, verify layout chaining works (`.liquid` layout can reference `.html` parent and vice versa).
- Verify that non-layout files (e.g., `.scss`, `.md`) in `_layouts/` are still ignored.

### Integration: al-folio site build
- Build `websites/al-folio/` with rustkyll, verify the homepage (`index.html`) contains `<!doctype html>` and `<body>`.
- Verify a blog post (e.g., `blog/2015/code/index.html`) contains the full HTML document structure with `<head>` and `<body>`.
- Run DOM comparison and record the new match count.

## Dependencies

- Issue #235 (al-folio site is set up)

## Log

### [SWE] 2026-03-29

- TDD Step 1: Wrote two failing tests in `src/template/layout.rs`:
  - `test_load_layouts_accepts_liquid_extension`: creates temp dir with `.html`, `.liquid`, and `.scss` files, verifies only `.html` and `.liquid` are loaded with correct names
  - `test_liquid_layout_chaining_with_html_parent`: verifies `.liquid` layout with `layout: default` front matter correctly chains to `.html` parent
- TDD Step 2: Both tests FAIL as expected:
  - `test_load_layouts_accepts_liquid_extension` panicked: "Should load .liquid layout as 'post'"
  - `test_liquid_layout_chaining_with_html_parent` panicked: "assertion failed: layouts.contains_key(\"page\")"
- TDD Step 3: Implemented fix in `src/template/layout.rs` (2 changes):
  1. Line 1018: Changed `if !filename.ends_with(".html")` to `if !filename.ends_with(".html") && !filename.ends_with(".liquid")`
  2. Lines 1025-1030: Changed layout name derivation to strip either `.html` or `.liquid` suffix
- TDD Step 4: Both tests PASS
- Full test suite: all 3190 tests pass, 0 fail
- Clippy: clean (no warnings from rustkyll crate)
- Fmt: clean
- DTC DOM: Verified 790/790 from clean committed code. My change is provably a no-op for DTC (only `.html` layouts exist). Dirty working tree shows 145/790 due to other in-progress issues' uncommitted changes.
- al-folio build: Layouts now load (13 `.liquid` files), but build fails later due to unimplemented `regex_replace` filter in bib.liquid template. This is expected -- layout loading was the blocker, other al-folio issues (#506-510) address remaining features.
- Files modified: `src/template/layout.rs`
