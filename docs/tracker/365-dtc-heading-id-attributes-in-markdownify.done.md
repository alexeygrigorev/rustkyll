# Issue 365: DTC missing heading id attributes in markdownify output

## Parent

Follow-up from #363 (RC-D).

## Problem

When markdownify produces `<h1>` or `<h3>` headings, Jekyll/kramdown adds `id` attributes (e.g., `id='then-do-your-stuff-with-the-pos-tags'`). Rustkyll's markdownify does not generate heading IDs.

### Root Cause

The main page-body rendering pipeline calls `kramdown::add_heading_ids()` in `kramdown::postprocess()` (at `src/kramdown.rs:700`), which generates slugified `id` attributes on all headings. However, the markdownify filter path (`markdown_to_html_for_filter` in `src/frontmatter.rs:728`) does NOT call `add_heading_ids`. As a result, headings produced by the `markdownify` Liquid filter lack `id` attributes.

### Fix

Call `add_heading_ids()` (with the appropriate `HeadingIdMode`) at the end of `markdown_to_html_for_filter`, after all existing postprocessing. The heading ID mode should respect the site's markdown engine configuration (kramdown vs CommonMarkGhPages), same as the main pipeline.

## Affected Pages

- `books/20211213-mastering-spacy.html` (1 diff) -- `<h1>` missing `id='then-do-your-stuff-with-the-pos-tags'`
- `books/20241017-build-large-language-model-from-scratch.html` (partial of 8 diffs) -- `<h3>` missing `id='user'`

## Dependencies

None. This is a self-contained change to the markdownify filter path.

## Acceptance Criteria

- [ ] `markdown_to_html_for_filter` generates `id` attributes on heading elements (`<h1>` through `<h6>`) matching Jekyll/kramdown slug rules (lowercase, spaces to hyphens, strip non-alphanumeric except hyphens)
- [ ] The heading `# Then do your stuff with the pos tags` produces `<h1 id="then-do-your-stuff-with-the-pos-tags">` in markdownify output
- [ ] The heading `### User` produces `<h3 id="user">` in markdownify output
- [ ] Heading ID mode respects the site's markdown engine setting (kramdown vs CommonMarkGhPages) -- no hardcoded mode
- [ ] No site-specific hardcoding -- the fix is generic to all headings in all markdownify output
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes with no regressions
- [ ] DTC DOM baseline: 776/790 -- match count must not drop below this
- [ ] DTC DOM diff for `books/20211213-mastering-spacy.html` no longer reports the `missing_attribute` for `id='then-do-your-stuff-with-the-pos-tags'`

## Test Scenarios

### Unit: markdownify heading IDs

- Parse `# Then do your stuff with the pos tags` through `markdown_to_html_for_filter`, verify output contains `id="then-do-your-stuff-with-the-pos-tags"`
- Parse `### User` through `markdown_to_html_for_filter`, verify output contains `id="user"`
- Parse `## Hello World` through `markdown_to_html_for_filter`, verify output contains `id="hello-world"`
- Parse markdown with no headings through `markdown_to_html_for_filter`, verify no regression (no id attributes on non-heading elements)

### Unit: special characters in heading IDs

- Parse `## It's a "test" & more!` through `markdown_to_html_for_filter`, verify `id` is slugified correctly (matches kramdown rules)
- Parse a heading with Unicode content (e.g., `## Cafe et Resume`) through `markdown_to_html_for_filter`, verify `id` is generated

### Unit: duplicate heading IDs

- Parse markdown with two identical headings (e.g., two `## Summary` headings) through `markdown_to_html_for_filter`, verify second gets `id="summary-1"` (kramdown dedup behavior)

### Integration: DTC output verification

- Build the DTC site with `./scripts/cargo-safe build`
- Inspect `books/20211213-mastering-spacy.html` output, verify the `<h1>` has `id="then-do-your-stuff-with-the-pos-tags"`
- Run DOM comparison, verify count does not drop below 776/790
- Verify `books/20211213-mastering-spacy.html` diff count drops from 1 to 0

## Priority

LOW

## Log

### [SWE] 2026-03-26

- TDD step 1: Wrote 7 unit tests in src/template/filters/markdownify.rs (h1/h2/h3 heading IDs, no-heading regression, special chars, unicode, duplicate dedup)
- TDD step 2: Ran tests -- 6 FAILED as expected (no id attributes on headings), 1 passed (no-heading regression)
- TDD step 3: Added `add_heading_ids()` call in `postprocess_for_filter_with_options` in src/kramdown.rs (5 lines), using same indent_lists -> HeadingIdMode mapping as the main postprocess pipeline
- TDD step 4: Ran tests -- all 7 PASS
- Updated 2 pre-existing regression tests that checked for `<h2>` (now `<h2 id=...>`) to use `<h2` prefix match instead
- Full test suite: 2822 passed, 3 failed (all 3 from other in-progress issue #372 uncommitted changes, not this issue)
- Clippy: 1 dead_code error from issue #372 uncommitted changes, not from this issue
- Fmt: 1 formatting diff from issue #372 uncommitted changes, not from this issue
- Build release: OK
- DTC DOM: 777/790 (up from baseline 776/790) -- mastering-spacy.html no longer in diff list
- build-large-language-model page: 7 diffs (down from 8), heading ID diff resolved
- Files modified: src/kramdown.rs (5 lines added), src/template/filters/markdownify.rs (7 new tests + 2 assertion fixes)

### [QA] 2026-03-26

- Code review: Clean 8-line change in `postprocess_for_filter_with_options` adds `add_heading_ids()` using `indent_lists` to select `HeadingIdMode`. No hardcoding, follows existing pattern from main pipeline.
- Tests: 7 new unit tests cover h1/h2/h3 IDs, no-heading regression, special chars, unicode, duplicate dedup. 2 pre-existing tests updated to prefix match.
- TDD verified: SWE log shows write tests -> 6 fail -> implement -> all pass cycle.
- `cargo test`: all pass (2825 passed, 0 failed)
- `cargo clippy -- -D warnings`: PASS (clean after full rebuild)
- `cargo fmt --check`: PASS (clean)
- DTC DOM: 778/790 (baseline 776/790, improved by +2)
- `books/20211213-mastering-spacy.html`: not in diff list (0 diffs, was 1)
- Acceptance criteria: all 11 criteria PASS

**VERDICT: PASS**
