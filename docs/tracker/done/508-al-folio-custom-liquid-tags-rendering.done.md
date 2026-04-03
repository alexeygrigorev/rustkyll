# Issue 508: Implement al-folio custom Liquid tags (tabs, details, quote, cite, etc.)

## Problem

24 al-folio pages contain raw Liquid syntax in the generated HTML because rustkyll does not implement several custom Liquid tags that al-folio defines via its Jekyll plugins. These tags appear as literal `{% tabs %}`, `{% details %}`, etc. in the output.

Affected custom tags (observed in al-folio output):

### Block tags (paired open/close)
- `{% tabs %}` / `{% endtabs %}` -- Tabbed content panels
- `{% details %}` / `{% enddetails %}` -- Collapsible details/summary blocks
- `{% quote %}` / `{% endquote %}` -- Styled blockquotes with attribution

### Inline tags (Jekyll Scholar plugin: `jekyll/scholar`)
- `{% cite %}` -- Bibliography citation references
- `{% reference %}` -- Full bibliography reference entries
- `{% bibliography %}` -- Render full bibliography from BibTeX

### Other inline tags
- `{% twitter %}` -- Embedded tweets
- `{% jupyter_notebook %}` -- Embedded Jupyter notebook HTML
- `{% post_url %}` -- Link to a post by filename (may already be partially supported)

### Filters
- `file_exists` -- Check if a file exists in the site source
- `bust_file_cache` -- Append cache-busting query parameter to file URLs

## Relationship to Issue 344

Issue #344 identified the same tags but has minimal acceptance criteria and no implementation detail. This issue supersedes #344 with concrete scope and testable criteria. Issue #344 should be closed in favor of this issue.

**Important note:** Many of the Liquid leaks observed in the current output (24 pages) are caused by issue #505 (missing `.liquid` layout support). When layouts are not found, rustkyll falls back to writing raw markdown-to-HTML content without Liquid processing. Once #505 is fixed, the leak count will drop significantly. The remaining leaks will be from genuine custom tags that need implementation.

## Scope

1. Implement the block tags (`tabs`, `details`, `quote`) as custom Liquid tags that produce the correct HTML structure.
2. Implement `cite` and `reference` tags as no-op or minimal stubs (full BibTeX support is out of scope; the goal is to stop Liquid leaks).
3. Implement `twitter` and `jupyter_notebook` as stubs that produce a placeholder or pass-through.
4. Implement `file_exists` and `bust_file_cache` filters.
5. Eliminate all raw Liquid syntax from al-folio output.

## Baseline

- al-folio Liquid leaks: 24 pages
- DTC DOM baseline: 790/790

## Acceptance Criteria

- [ ] No al-folio page contains raw `{% tabs %}`, `{% details %}`, `{% quote %}`, `{% cite %}`, `{% reference %}`, `{% twitter %}`, or `{% jupyter_notebook %}` in the generated HTML.
- [ ] `{% tabs %}` produces a `<ul>` with tab navigation and `<div>` panels (matching al-folio's expected output).
- [ ] `{% details %}` produces `<details><summary>` HTML elements.
- [ ] `{% quote %}` produces a styled `<blockquote>` with optional attribution.
- [ ] `{% cite %}` and `{% reference %}` produce reasonable output (even if simplified) without leaking Liquid.
- [ ] `file_exists` filter returns `true`/`false` based on whether the file exists in the source directory.
- [ ] `bust_file_cache` filter appends a cache-busting query parameter.
- [ ] al-folio Liquid leak count drops from 24 to 0.
- [ ] DTC DOM match count does not drop below 790/790.
- [ ] `cargo build` compiles without errors; `cargo clippy` clean; `cargo fmt` clean.

## Test Scenarios

### Unit: tabs tag
- Parse `{% tabs group %} {% tab group label %} content {% endtab %} {% endtabs %}`, verify output contains tab navigation and panels.

### Unit: details tag
- Parse `{% details Summary text %} body content {% enddetails %}`, verify output is `<details><summary>Summary text</summary>body content</details>`.

### Unit: quote tag
- Parse `{% quote Author %} quote text {% endquote %}`, verify styled blockquote with attribution.

### Unit: cite/reference tags
- Parse `{% cite einstein2015 %}`, verify no Liquid leak (output can be a stub like `[einstein2015]`).

### Unit: filters
- Test `file_exists` returns true for existing files and false otherwise.
- Test `bust_file_cache` appends query parameter to URLs.

### Integration: al-folio build
- Build al-folio and grep output for raw `{%` -- verify zero matches in HTML content.
- Verify blog/2024/tabs page has actual tab HTML structure.

## Dependencies

- Issue #235 (al-folio site is set up)
- Issue #505 (layouts must be applied for full page context)

## Log

### [SWE] 2026-04-02

**Context:** Two tags were FULLY IMPLEMENTED but NOT REGISTERED (dead code):
- `src/template/details_tag.rs` -- `{% details %}...{% enddetails %}` block tag (207 lines, 5 inline tests)
- `src/template/file_exists_tag.rs` -- `{% file_exists path %}` inline tag (179 lines, 5 inline tests)

The remaining tags (tabs, quote, cite, reference, bibliography, jupyter_notebook, twitter, bust_file_cache, social_links, github_edit_link) were already registered as no-ops in `noop_tags.rs`.

**Fix 1: Wire up details_tag and file_exists_tag**

- Wrote 5 integration tests in `tests/test_issue_508_custom_tags.rs`:
  - `test_details_block_produces_html`
  - `test_details_block_unicode_content`
  - `test_file_exists_tag_true`
  - `test_file_exists_tag_false`
  - `test_file_exists_in_capture_block`
- Ran tests: FAILS -- all 5 fail with "unknown Liquid tag 'details'" / "unknown Liquid tag 'file_exists'" rendered as empty strings
- Implementation:
  - Added `pub mod details_tag;` and `pub mod file_exists_tag;` to `src/template/mod.rs`
  - Registered `DetailsBlock` (block) and `FileExistsTag` (tag) in all 5 builder spots in `src/template/engine.rs`
  - Fixed `details_tag.rs` compilation: changed `crate::markdown::render_markdown` to `crate::frontmatter::markdown_to_html`
  - Fixed `details_tag.rs` compilation: wrapped `block.parse_all()` result with `liquid_core::runtime::Template::new()`
- Ran tests: PASSES -- all 5 integration tests + 5 details inline tests + 5 file_exists inline tests pass

**Fix 2: Pre-existing clippy type_complexity warning**

- Fixed `src/template/filters/where_filter.rs`: extracted type alias `WhereIndexMap` for complex HashMap type

**Summary:**
- Files modified: `src/template/mod.rs`, `src/template/engine.rs`, `src/template/details_tag.rs`, `src/template/filters/where_filter.rs`
- Files created: `tests/test_issue_508_custom_tags.rs`
- Tests added: 5 integration tests (details + file_exists end-to-end)
- Pre-existing tests now compiling: 10 inline tests (5 details + 5 file_exists)
- Build results: 3659+ tests pass, 0 fail, clippy clean, fmt clean
- DTC DOM: 596/790 matched (same as baseline -- no regression)
- DTC build time: 0.58s (under 1.0s threshold)
- Note: Issue #505 (al-folio liquid layout extension) is still in-progress, not yet moved to done

### [QA] 2026-04-03

**Tests:**
- 5 integration tests (test_issue_508_custom_tags.rs): ALL PASS
- 5 unit tests (details_tag::tests): ALL PASS
- 5 unit tests (file_exists_tag::tests): ALL PASS
- 1 pre-existing failure (test_link_tag_pretty_permalink_with_anchor) -- unrelated to this issue, from other uncommitted changes in working tree
- Clippy: clean (no warnings from our code)
- Fmt: clean

**DTC DOM regression check:**
- Baseline at HEAD (without SWE changes): 596/790, 255 total diffs
- After SWE changes: 596/790, 255 total diffs
- No regression. Note: the issue file states "DTC DOM baseline: 790/790" but the actual committed codebase baseline is 596/790. The SWE correctly reported the real number.
- DTC build time: 0.576s (under 1.0s threshold)

**TDD compliance:** PASS -- SWE log shows test-first cycle: tests written, verified FAILS with "unknown Liquid tag", then implementation, then verified PASSES.

**Tag registration:** Verified DetailsBlock and FileExistsTag are registered in all 5 builder spots in engine.rs (lines 670, 720, 786, 940, 1114 for DetailsBlock; lines 671, 721, 787, 941, 1115 for FileExistsTag).

**Acceptance criteria (scoped to SWE's work -- wiring up 2 tags):**
- `{% details %}` produces `<details><summary>` HTML elements: PASS
- `file_exists` tag returns true/false based on file existence: PASS
- `cargo build` compiles, `clippy` clean, `fmt` clean: PASS
- DTC DOM no regression: PASS (596/790 unchanged)

**Acceptance criteria NOT addressed (remaining work for this issue):**
- `{% tabs %}` producing `<ul>` tab navigation -- still a no-op
- `{% quote %}` producing styled `<blockquote>` -- still a no-op
- `{% cite %}` / `{% reference %}` producing reasonable output -- still no-ops
- `bust_file_cache` filter -- still a no-op
- al-folio Liquid leak count dropping to 0 -- not verified/achieved
- No raw Liquid syntax in al-folio output -- not verified for all tags

**VERDICT: PASS** -- The scoped work (wiring up details_tag and file_exists_tag) is correctly implemented, tested, and regression-free. The broader issue criteria (tabs, quote, cite, bust_file_cache, zero al-folio leaks) remain open and should be tracked as continuing work on this issue or split into follow-up issues by PM.

### [PM] 2026-04-02 22:30
- Reviewed diff: 8 files changed (src/template/mod.rs, engine.rs, details_tag.rs, file_exists_tag.rs, filters/where_filter.rs, collection.rs, generator.rs + 1 new test file)
- Output verification: details tag produces correct `<details><summary>` HTML; file_exists returns true/false correctly; both verified via 5 integration tests + 10 unit tests
- Results verified: DTC DOM 596/790, no regression from baseline; 3659+ tests pass; clippy clean
- Acceptance criteria met:
  - `{% details %}` produces `<details><summary>` HTML: MET
  - `file_exists` tag returns true/false: MET
  - `cargo build` / `clippy` / `fmt` clean: MET
  - DTC DOM no regression: MET (596/790)
- Acceptance criteria descoped to #545:
  - `{% tabs %}` producing tab navigation HTML
  - `{% quote %}` producing styled blockquote
  - `{% cite %}` / `{% reference %}` producing stub output
  - `bust_file_cache` filter
  - al-folio Liquid leak count dropping to 0
- Follow-up issues created: #545 (al-folio remaining custom tags: tabs, quote, cite, bust_file_cache)
- VERDICT: ACCEPT
