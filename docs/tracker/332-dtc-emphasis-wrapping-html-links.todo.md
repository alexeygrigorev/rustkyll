# Issue 332: DTC emphasis wrapping inline HTML links (pulldown-cmark context bug)

## Problem

On `blog/interview-with-valerii-chetvertakov.html`, patterns like `*<a href="...">EV Connect, Inc.</a>, text* *<a href="...">Schneider Electric</a>, text"*` are output as literal `*` characters instead of `<em>` tags. Jekyll wraps each `*...*` span in `<em>`.

This was originally Problem 2 in issue 275. The bug only reproduces in the context of the full DTC blog post -- simplified unit tests pass correctly. This is a context-dependent pulldown-cmark emphasis parsing bug, similar to Problem 1 (fixed in issue 275 via postprocessing) but requiring a different approach.

## Affected pages

- `blog/interview-with-valerii-chetvertakov.html` -- 17 diffs caused by literal `*` instead of `<em>`

## Root Cause

When `*...*` spans contain `<a>` tags (from already-processed inline HTML), the emphasis parser fails to find the closing `*` and falls back to literal output. This only occurs in specific document contexts -- the full file content triggers the pulldown-cmark misparsing.

## Dependencies

- Issue 275 (done) -- Problem 1 fix is already merged

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `./scripts/cargo-safe test` passes
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] Input containing `*<a href="url">Link</a>, trailing text*` in the context of the full DTC blog post produces `<em>` tags, not literal `*`
- [ ] DTC DOM comparison: `blog/interview-with-valerii-chetvertakov.html` emphasis diffs resolved (17 diffs reduced significantly)
- [ ] DTC DOM comparison overall: no regression from current 751/790
- [ ] No regressions on other sites
- [ ] Tests use actual DTC file content to reproduce the context-dependent bug (not just simplified inputs)

## Notes

- The simple unit test `test_issue275b_emphasis_wrapping_html_link` passes in isolation; the bug requires full document context
- May need a preprocessing or postprocessing approach similar to `fix_nested_emphasis_tags()` from issue 275
- Alternatively, may need to preprocess the markdown to escape or transform the problematic patterns before passing to pulldown-cmark
