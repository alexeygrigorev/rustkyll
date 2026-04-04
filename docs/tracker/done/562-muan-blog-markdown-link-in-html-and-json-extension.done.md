# Issue 562: muan-blog markdown link not rendered in HTML list + .json page rendered as .html

## Problem

Two small muan-blog issues:

### A. Markdown link syntax not rendered inside HTML `<li>` (1 page, 2 diffs)

**posts/leaving-github.html**: The source contains a markdown link inside an HTML `<li>` element:
```
<li>[text](mailto:hi+bye@muan.co?subject=...)</li>
```

Jekyll renders the markdown link syntax into an `<a>` tag. Rustkyll passes it through as raw text: `[text](mailto:...)`. This is because CommonMark treats content inside HTML blocks as raw HTML, not parsing markdown within them. However, Jekyll's CommonMarkGhPages plugin apparently does process markdown inside certain HTML elements.

### B. .json page rendered as .html (1 only-rustkyll file)

**pages/acitivitypub.html**: The source file is `_pages/acitivitypub.json` (a JSON file with front matter). Jekyll outputs this as `pages/acitivitypub.json`. Rustkyll outputs it as `pages/acitivitypub.html`, incorrectly changing the extension.

## Affected Site

- muan-blog: 1 page with diffs (leaving-github), 1 extra file (activitypub)
- These are minor edge cases

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests
- [ ] Pages with `.json` extension and front matter output as `.json` not `.html`
- [ ] muan-blog no longer generates `pages/acitivitypub.html` (should be `.json`)
- [ ] DTC DOM match count must not drop below 790/790
- [ ] Markdown inside HTML `<li>` elements renders as links when using CommonMarkGhPages (stretch goal -- may require deeper investigation)

## Test Scenarios

### Unit: file extension preservation
- A `.json` file with front matter should output with `.json` extension
- A `.xml` file with front matter should output with `.xml` extension
- A `.html` file with front matter should output with `.html` extension

### Unit: markdown in HTML elements (stretch)
- Parse HTML containing `<li>[text](url)</li>` with CommonMark
- Investigate whether this should render the markdown link

### Integration: muan-blog build
- Build muan-blog and verify `pages/acitivitypub.json` exists (not `.html`)
- Verify no `pages/acitivitypub.html` in output

## Dependencies

None

## DTC DOM Baseline

790/790 (100%)

## Log

### [PM] 2026-04-02 10:00
- Groomed from batch site analysis
- Two minor muan-blog issues grouped together
- The markdown-in-HTML issue may be hard to fix without breaking other things
- The .json extension issue should be straightforward

### [SWE] 2026-04-02

**Fix 1: .json extension preservation for collection items**
- Wrote test: test_json_collection_item_preserves_extension (src/collection.rs)
- Ran test: FAILS -- URL should end with .json, got: /pages/acitivitypub
- Implemented fix in src/collection.rs: when permalink pattern contains :output_ext and file is non-markdown/non-html, append source extension to URL
- Ran test: PASSES

**Additional tests:**
- test_xml_collection_item_preserves_extension: .xml files also preserved
- test_html_collection_item_keeps_html_extension: .html files unaffected

**Markdown-in-HTML stretch goal:**
- Investigated the leaving-github.html diffs. The issue is a broken markdown link `[text](mailto:...?subject=...&body=...[reasons].)` where `[reasons]` inside the URL breaks CommonMark link parsing. Jekyll's CommonMarkGhPages autolink extension partially renders the `mailto:` as an autolink `<a>` tag, which Rustkyll doesn't do in this context. This is a deep CommonMark edge case and is left as a stretch goal per the issue spec.
- leaving-github.html still has 2 diffs (the partial mailto autolink) -- these are pre-existing edge cases

**Summary:**
- Files modified: src/collection.rs
- Tests added: 3 (JSON/XML/HTML extension preservation)
- Build results: 3866+ tests pass, 0 fail, clippy clean, fmt clean
- DTC DOM: 790/790 (0 total diffs) -- baseline maintained
- muan-blog: pages/acitivitypub.json now output correctly (was .html)
- DTC build time: 0.882s

### [PM] 2026-04-02 14:30
- Reviewed diff: 1 file changed (collection.rs +113 lines including 3 tests)
- Output verification: DTC DOM 790/790 confirmed via recount-all-dom.sh
- Tests verified: 4290 pass, 0 fail; 3 extension-preservation tests (json, xml, html) all pass
- Clippy: clean
- Acceptance criteria: all met -- .json extension preserved, .xml also handled, .html unchanged, DTC 790/790
- Stretch goal (markdown-in-HTML li): correctly deferred -- documented as deep CommonMark edge case
- VERDICT: ACCEPT
