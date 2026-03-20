# Issue 254: Support :name permalink placeholder

## Problem

Collection permalinks using `/:name/` are output literally instead of being resolved to the document slug. Found in jekyll-vitepress-theme, where the config uses:

```yaml
collections:
  introduction:
    output: true
    permalink: "/:name/"
```

All 16 collection pages end up at the literal path `/:name/index.html` instead of resolving to each document's slug (e.g., `/getting-started/index.html`).

## Root Cause

In `src/collection.rs`, the `generate_url_with_context` function (around line 195) has a replacement chain for permalink placeholders: `:collection`, `:slug`, `:title`, `:year`, `:month`, `:day`, `:short_year`, `:i_month`, `:i_day`, `:categories`, `:path`. The `:name` placeholder is missing from this chain.

## Jekyll Semantics

In Jekyll, `:name` is the filename-derived slug (the filename without the date prefix and without the extension). For non-dated documents (like collection pages), `:name` is equivalent to `:title` / `:slug` -- it is the entire filename stem. For dated posts, `:name` strips the `YYYY-MM-DD-` prefix.

Since `PermalinkContext.title` already holds the filename-derived slug (with date prefix stripped), the fix is to add `.replace(":name", &ctx.title)` to the existing replacement chain.

## Fix Location

File: `src/collection.rs`, function `generate_url_with_context`, the `.replace(...)` chain starting at line 195. Add `:name` replacement. Also update the doc comment on line 152 to list `:name` in the supported placeholders.

## Dependencies

None. The permalink resolution infrastructure is already in place.

## Acceptance Criteria

- [ ] `:name` placeholder in permalink patterns resolves to the document's filename-derived slug (same value as `:title`)
- [ ] The doc comment on `generate_url_with_context` lists `:name` as a supported placeholder
- [ ] `cargo build` compiles without errors
- [ ] `cargo fmt` reports no formatting issues
- [ ] `cargo clippy -- -D warnings` passes cleanly
- [ ] All existing tests pass (no regressions)
- [ ] New unit tests added for `:name` placeholder resolution (see test scenarios below)
- [ ] Building jekyll-vitepress-theme produces correct per-document URLs instead of literal `/:name/` paths (output verification)

## Test Scenarios

### Unit: :name placeholder in generate_url_with_context

1. **Basic :name resolution** -- Call `generate_url_with_context` with pattern `"/:name/"` and title `"getting-started"`. Verify output is `"/getting-started/"`.
2. **:name with collection prefix** -- Call with pattern `"/:collection/:name/"` and collection `"introduction"`, title `"overview"`. Verify output is `"/introduction/overview/"`.
3. **:name with .html suffix** -- Call with pattern `"/:name.html"` and title `"my-page"`. Verify output is `"/my-page.html"`.
4. **:name equivalence with :title** -- For the same `PermalinkContext`, verify that `"/:name/"` and `"/:title/"` produce identical output.
5. **:name with date context** -- Call with pattern `"/:year/:name/"`, date `"2024-01-15"`, title `"my-post"`. Verify output is `"/2024/my-post/"`.

### Integration: jekyll-vitepress-theme output verification

6. **Build vitepress theme and verify no URL conflict warning** -- Build the jekyll-vitepress-theme site (if the source is available at `websites/jekyll-vitepress-theme`) and verify the "Conflict: The URL '/:name/'" warning is gone. Each collection page should have a unique URL derived from its filename slug.

## Output Verification

After the fix, building jekyll-vitepress-theme should produce:
- 16 separate collection page directories (e.g., `/getting-started/index.html`, `/overview/index.html`, `/code-blocks/index.html`, etc.) instead of a single `/:name/index.html`
- No "Conflict" warning in build output
- The sitemap should list 16+ unique URLs for collection pages

## Log

### [SWE] 2026-03-20
- Wrote 5 unit tests for `:name` placeholder in `src/collection.rs` (test_name_placeholder_basic, test_name_placeholder_with_collection, test_name_placeholder_with_html_suffix, test_name_placeholder_equivalence_with_title, test_name_placeholder_with_date)
- Ran tests: all 5 FAIL as expected -- output shows literal `/:name/` instead of resolved slug
- Implemented fix: added `.replace(":name", &ctx.title)` to the replacement chain in `generate_url_with_context` (line 198)
- Updated doc comment on `generate_url` (line 152) to list `:name` as supported placeholder
- Ran tests: all 5 PASS -- `:name` now resolves to document slug
- Full test suite: 1921 lib tests + all integration tests pass, 0 failures
- Clippy: pre-existing failure in third-party `liquid-core` dependency (not from our changes); our code is clean
- Fmt: `src/collection.rs` is clean; pre-existing fmt diffs exist in other files from concurrent work
- Files modified: `src/collection.rs`
