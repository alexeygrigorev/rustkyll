# Issue 517: Chirpy text-and-typography page has ~100 structural diffs

## Problem

The `posts/text-and-typography/index.html` page on the chirpy test site has 109 total
diffs in DOM comparison. This page exercises nearly every advanced feature of the Chirpy
theme: IAL classes, image post-processing (lqip/lazy loading), code block restructuring,
heading anchors, mermaid diagrams, and math equations.

Most diffs stem from root causes tracked in other issues:

1. **Block IAL classes** (~20 diffs): kramdown IALs like `{: .prompt-tip }` not applied.
   Tracked by #505 (in-progress).
2. **Chirpy refactor-content.html image handling** (~15 diffs): The include extracts `lqip`
   from `<img>`, wraps images in `<a class="popup img-link">`, converts `src=` to `data-src=`.
   This depends on exact attribute ordering matching Jekyll output.
3. **Code block structure** (~30 diffs): The include removes inner `<pre>` wrapper, adds
   code-header divs with language labels. Depends on #471 (syntax highlight structure).
4. **Heading anchors** (~10 diffs): The include generates anchor links for h2-h5. Depends
   on heading IDs matching Jekyll exactly.
5. **Mermaid/math** (~10 diffs): Fenced code blocks with `mermaid` language and LaTeX.
6. **SEO meta tags** (~4 diffs): og:image contains raw template content instead of resolved
   path. Tracked by #514 (in-progress).

## Root Cause

This is a compound issue. The individual root causes are in:
- `src/kramdown_parser/parser.rs` -- IAL application to block elements
- `src/syntax.rs` / syntax highlighting -- code block HTML structure
- `src/kramdown_parser/html.rs` -- heading ID generation
- `src/template/engine.rs` -- SEO tag template rendering

## Scope

This is a **tracking/investigation issue**. The engineer must:
1. Wait for blocking dependencies to land
2. Rebuild chirpy and re-run DOM comparison
3. Categorize remaining diffs
4. Either fix remaining issues directly or create follow-up issues

## Dependencies (BLOCKING -- all must be .done.md first)

- Issue #505 (block IAL class application) -- currently in-progress
- Issue #471 (syntax highlighting token mismatches) -- currently in-progress
- Issue #514 (SEO tag hash image frontmatter) -- currently in-progress

**This issue CANNOT be started until all three dependencies are done.**

## Baseline

- DTC: 790/790 (must not regress)
- Chirpy: 12/17 pages match (this page is one of the 5 that differ)
- This page: 109 diffs currently

## Acceptance Criteria

- [ ] All three blocking dependencies (#505, #471, #514) are in .done.md status
- [ ] Chirpy site rebuilt with `./scripts/cargo-safe build` using updated code
- [ ] DOM comparison re-run on chirpy site after rebuild
- [ ] Remaining diff count for `posts/text-and-typography/index.html` documented
- [ ] Each remaining diff category classified as one of:
  - (a) Fixed in this issue
  - (b) Follow-up issue created in `docs/tracker/` with specific scope
  - (c) Documented as unfixable (theme-specific Ruby hook behavior)
- [ ] Total diffs on this page reduced from 109 to under 50
- [ ] DTC DOM baseline remains at 790/790
- [ ] No regression on any other chirpy page (12/17 must stay or improve)
- [ ] `cargo test` passes

## Test Scenarios

### Investigation: DOM diff analysis
- Build chirpy after dependencies land, count diffs on text-and-typography page
- Compare heading IDs between Jekyll and rustkyll output for this page
- Compare code block HTML structure for at least 2 code examples on this page
- Verify IAL classes appear on headings and blockquotes after #505 lands
- Verify og:image meta tag is correct after #514 lands

### Output verification
- Build chirpy site and inspect `posts/text-and-typography/index.html`
- Count actual DOM diffs using the comparison tool
- Spot-check at least 3 specific diff categories to verify they are resolved or tracked
