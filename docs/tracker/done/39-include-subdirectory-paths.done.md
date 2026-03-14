# Issue 39: Support Include Paths with Subdirectory Separators

## Problem

Cross-site testing (Issue 32) revealed that `{% include %}` tags with path separators (`/`) in the filename are not parsed correctly by the Liquid template engine.

Examples:
- `{% include icons/icons.html %}` -- fails to parse
- `{% include course-structured-data/data-engineering-zoomcamp-structured-data.html %}` -- fails to parse

The Liquid parser treats the `/` as an unexpected character instead of part of the filename. This produces 6 warnings during DTC site builds (all in the `course-structured-data/` subdirectory of `_includes/`).

## Root Cause

In `src/template/include_tag.rs`, the `parse` method calls `arguments.expect_next()` which returns a single token. The Liquid tokenizer splits at `/`, so `course-structured-data/file.html` becomes multiple tokens: `course-structured-data`, `/`, `file.html`. The parser only consumes the first token as the filename.

The include file loading (`load_includes_recursive` in `engine.rs`) already correctly registers subdirectory includes with `/` in their keys (e.g., `"course-structured-data/data-engineering-zoomcamp-structured-data.html"`). The fix is entirely in the parse side.

## Found In

- `DataTalksClub/docs` -- uses `{% include icons/icons.html %}`
- `DataTalksClub/datatalksclub.github.io` -- uses `{% include course-structured-data/*.html %}` (6 posts affected, currently produce warnings)

## Requirements

- Update the include tag parsing in `src/template/include_tag.rs` to allow `/` in include file paths
- After consuming the initial filename token, check if the next token is `/` and if so, continue consuming tokens to build the full path (e.g., `subdir/file.html`)
- Support multiple levels of nesting (e.g., `a/b/c.html`)
- Continue to support simple filenames without `/` (no regression)
- Continue to support parameters after the filename (e.g., `{% include subdir/file.html param="value" %}`)
- Resolve include paths relative to the `_includes/` directory (already works in `load_includes`)

## Dependencies

- None (include loading already supports subdirectories)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `{% include subdir/file.html %}` parses and renders the correct partial from `_includes/subdir/file.html`
- [ ] `{% include subdir/file.html param="value" %}` works with parameters
- [ ] `{% include deeply/nested/file.html %}` works with multiple path segments
- [ ] Simple includes like `{% include simple.html %}` continue to work (no regression)
- [ ] Building the DTC site produces no warnings related to include path parsing for the 6 `course-structured-data/` includes
- [ ] `cargo test` passes with all new and existing tests

## Test Scenarios

### Unit: Subdirectory include parsing

- Parse and render `{% include subdir/file.html %}` with a matching partial registered as `"subdir/file.html"` -- verify output matches the partial content
- Parse and render `{% include a/b/c.html %}` with deeply nested path -- verify correct partial is resolved
- Parse and render `{% include subdir/file.html param="hello" %}` -- verify both the partial renders and the parameter is accessible via `include.param`
- Parse and render `{% include subdir/file.html %}` where the partial uses `{{ include.nonexistent }}` -- verify lenient behavior (Nil, no error)

### Unit: No regression on existing include behavior

- Verify `{% include simple.html %}` still works (existing test)
- Verify `{% include ev.html event="test" speakers=false %}` still works with parameters (existing test)
- Verify `{% include nonexistent.html %}` still produces an error (existing test)

### Integration: DTC site includes

- Load includes from `datatalksclub.github.io/_includes/` and verify the engine can parse a template containing `{% include course-structured-data/data-engineering-zoomcamp-structured-data.html %}` without error
- Render a template using one of the `course-structured-data/` includes and verify non-empty output
