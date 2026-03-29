# Issue 473: muan-blog URL slug/permalink differences
## Problem
Internal post links use date-prefixed slugs (`/posts/2024-11-02-javascript`) vs Jekyll's simple slugs (`/posts/javascript`). 3 files.
## Affected Sites
- muan-blog (3 files)
## Baseline
DTC 790/790. Must not regress.

## Log

### [SWE] 2026-03-29
- Root cause: `preprocess_jekyll_tags` in engine.rs stripped `_posts/` prefix but kept date prefix in filename
- Wrote 8 failing tests: 5 for post link date stripping with various permalink patterns, 3 for non-post/non-collection link preservation
- Ran tests: FAILS as expected (produced `/posts/2024-11-02-javascript` instead of `/posts/javascript`)
- Implemented fix:
  - Added `post_permalink: RwLock<Option<String>>` field to `TemplateEngine`
  - Added `set_post_permalink()` to `TemplateEngine` and `LayoutEngine`
  - Modified `preprocess_jekyll_tags` to accept `post_permalink: Option<&str>`
  - For `{% link _posts/... %}` paths: parse filename with `parse_post_filename`, apply permalink via `generate_url_with_context`
  - For non-posts links: behavior unchanged
  - Hooked up in `main.rs`: `layout_engine.set_post_permalink(&config.permalink)`
- Ran tests: PASSES -- all 3102 tests pass, 8 new tests for issue 473
- Clippy clean, fmt clean
- muan-blog DOM: 2198/2218 (up from 2197 baseline, +1 improvement)
  - goodies.html: now correctly links to `/posts/javascript`
  - reparations.html: now correctly links to `/posts/thoughts-on-reparations`
  - thoughts-on-reparations.html: now correctly links to `/posts/reparations`
  - Remaining diffs in reparations.html and thoughts-on-reparations.html are unrelated (URL encoding, br/p tag differences)
- DTC DOM: cannot verify in isolation due to other in-progress changes in working tree; changes only touch `{% link _posts/... %}` which DTC site does not use
- Files modified: src/template/engine.rs, src/template/layout.rs, src/main.rs
