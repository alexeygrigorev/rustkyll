# Issue 576: Kramdown abbreviations not applied during site builds

## Problem

Kramdown abbreviation definitions (`*[CSS]: Cascading Style Sheets`) work correctly in unit tests but are NOT applied during actual site builds. The abbreviation definition line is output as literal text, and matching text in the document is not wrapped in `<abbr>` tags.

**Jekyll output (expected):**
```html
<p>The abbreviation <abbr title="Cascading Style Sheets">CSS</abbr> stands for "Cascading Style Sheets".</p>
```

**Rustkyll output (actual):**
```html
<p>The abbreviation CSS stands for "Cascading Style Sheets".</p>
<p>*[CSS]: Cascading Style Sheets</p>
```

Two bugs:
1. The abbreviation definition `*[CSS]: ...` is rendered as a paragraph instead of being stripped
2. The text "CSS" in the content is not wrapped in `<abbr title="...">`

## Impact

- **Hydeout**: `markup/2012/01/11/markup-html-elements-and-formatting.html` -- abbreviation not rendered (contributes to 32 diffs on this page)
- Any site using kramdown abbreviation syntax will have the same problem
- The unit tests (kramdown_span_abbreviations_abbrev, _abbrev_defs, _abbrev_in_html) all pass, so the parsing logic exists but is not wired into the build pipeline

## Root Cause

The kramdown parser's abbreviation extraction and replacement works in isolation (unit tests), but the code path used during site builds (likely `markdown_to_html` or equivalent) does not invoke the abbreviation processing step, or the abbreviation definitions are not being passed from the document extraction phase to the rendering phase.

## Scope

- Find where the kramdown parser is invoked during site builds
- Ensure abbreviation definitions are extracted from the document
- Ensure abbreviation definitions are stripped from output
- Ensure matching text is wrapped in `<abbr title="Full Text">ABBR</abbr>`
- Do NOT change the unit test infrastructure (tests already pass)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests plus new ones
- [ ] For input containing `*[CSS]: Cascading Style Sheets`, the definition line is NOT present in HTML output
- [ ] For input containing `*[CSS]: Cascading Style Sheets` and text "CSS", the output contains `<abbr title="Cascading Style Sheets">CSS</abbr>`
- [ ] Hydeout `markup/2012/01/11/markup-html-elements-and-formatting.html` contains `<abbr>` tag in output
- [ ] DTC DOM match count must not drop below 790/790

## Test Scenarios

### Unit: Abbreviation in full pipeline
- Create a markdown string with `*[HTML]: Hyper Text Markup Language` and text "HTML is great", parse through the full site-build markdown pipeline, verify output contains `<abbr title="Hyper Text Markup Language">HTML</abbr>`
- Verify the abbreviation definition line is not present in the output HTML

### Integration: Hydeout site build
- Build hydeout site
- Check `markup/2012/01/11/markup-html-elements-and-formatting.html` for `<abbr>` tag
- Verify abbreviation definition `*[CSS]: ...` is not rendered as paragraph

## Dependencies

None.

## DTC DOM Baseline

790/790 (must not drop)

## Log

### [SWE] 2026-04-02

**Root Cause:** The `markdown_to_html` and `markdown_to_html_with_options` functions in `src/frontmatter.rs` use pulldown-cmark for markdown parsing, followed by kramdown postprocessing. Neither step extracted or applied abbreviation definitions. The kramdown_parser module has full abbreviation support but it's only used in its own unit test pipeline, not in the main site build pipeline.

**Fix: Wire abbreviation extraction and application into the build pipeline**

- Wrote 5 tests in `src/frontmatter.rs`:
  - `test_markdown_to_html_abbreviation_applied` -- verifies CSS wrapped in `<abbr>`
  - `test_markdown_to_html_abbreviation_definition_stripped` -- verifies `*[HTML]:` removed
  - `test_markdown_to_html_abbreviation_unicode` -- verifies unicode title in `<abbr>`
  - `test_markdown_to_html_multiple_abbreviations` -- verifies multiple abbreviations
  - `test_markdown_to_html_abbreviation_empty_title` -- verifies empty title produces `<abbr>` without title attr
- Ran tests: ALL 5 FAIL as expected (abbreviation definitions rendered as paragraphs, no `<abbr>` tags)
- Implemented fix:
  - Added `extract_abbreviation_definitions()` and `apply_abbreviations()` in `src/kramdown.rs`
  - Wired into `markdown_to_html()` (line ~623) -- extract before preprocessing, apply after postprocessing
  - Wired into `markdown_to_html_with_options()` (line ~870) -- same pattern, with feature pre-scan (`*[` check)
- Ran tests: ALL 5 PASS

**Verification:**
- Full test suite: 3945 passed, 0 failed, 4 ignored
- Clippy: clean (no warnings from our code)
- Fmt: clean
- DTC DOM: 790/790 with 0 total diffs (unchanged)
- DTC build time: 0.844s (under 1.0s threshold)
- Hydeout site build: `<abbr title="Cascading Style Sheets">CSS</abbr>` present in output, matches Jekyll exactly
- Abbreviation definition `*[CSS]:` stripped from output

**Files modified:**
- `src/kramdown.rs` -- added `AbbreviationDef`, `extract_abbreviation_definitions()`, `apply_abbreviations()`, `escape_html_attr_for_abbr()`, `replace_abbr_outside_tags()`
- `src/frontmatter.rs` -- wired abbreviation extraction/application into `markdown_to_html()` and `markdown_to_html_with_options()`; added 5 tests

### [PM] 2026-04-02 16:30
- Reviewed diff: 3 files changed (244 insertions, 7 deletions)
- Output verification: Built DTC site and Hydeout site, inspected generated HTML
  - Hydeout `markup/2012/01/11/markup-html-elements-and-formatting.html` contains `<abbr title="Cascading Style Sheets">CSS</abbr>` -- matches Jekyll
  - Abbreviation definition `*[CSS]:` correctly stripped from output
  - DTC site titles, metadata, and content unaffected by abbreviation feature
- Results verified: DTC DOM 790/790 (no regression), Hydeout abbreviations render correctly
- Code review: clean implementation -- abbreviation extraction before markdown processing, application after HTML postprocessing; word boundary checks prevent partial matches; longest-first sorting avoids overlap issues; proper HTML attribute escaping
- Tests: 5 new tests covering basic application, definition stripping, unicode, multiple abbreviations, empty title -- all meaningful
- Acceptance criteria: all met
- Follow-up issues created: none
- VERDICT: ACCEPT
