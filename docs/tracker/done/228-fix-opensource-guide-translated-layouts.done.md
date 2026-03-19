# Issue 228: Fix opensource-guide markdown="1" and heading ID rendering

## Problem

opensource-guide matches 0/388 pages (0% exact match). The original issue assumed layouts were not applied, but investigation shows layouts ARE applied -- the page structure (body > main > article) is correct. The actual root causes are content rendering issues, primarily:

1. **`markdown="1"` attribute not processed** (~545 diffs across 260+ aside elements): kramdown's `markdown="1"` HTML attribute tells the processor to render markdown inside that HTML element. Rustkyll passes the attribute through literally and does not process the markdown content within. This causes `<aside markdown="1">` to appear in output with raw markdown inside instead of rendered HTML. Effects: `markdown='1'` appears as extra attribute, `<img>` appears where `<p>` is expected (because the img markdown syntax is not processed), and `<p>` elements are missing.

2. **Heading IDs for Arabic script** (~34 heading href diffs): kramdown's default auto_id generation strips characters that are not `[a-zA-Z0-9]` before slugifying. For pure-Arabic headings like `## ما معنى أن تكون مسؤول عن مشروع`, kramdown generates `id="section"` (falling back because all Arabic chars are stripped), with duplicates getting `section-1`, `section-2`, etc. Rustkyll's `slugify()` preserves Unicode alphabetic characters via `char::is_alphanumeric()`, generating Arabic-text slugs instead.

3. **`{#id}` heading ID syntax not parsed** (~15 occurrences in 9 files): Some Arabic articles use kramdown's explicit heading ID syntax like `## تعلم قول لا {#تعلم-قول-لا}`. Kramdown strips the `{#id}` from the heading text and uses it as the heading's `id` attribute. Rustkyll leaves `{#id}` as literal text in the heading, which then gets included in the slug, causing doubled IDs like `#تعلم-قول-لا-تعلم-قول-لا`.

## Out of Scope (separate issues needed)

These root causes also affect opensource-guide but are cross-site issues that should be tracked separately:

- **Jekyll version/timestamp diffs** (~728 diffs): Cosmetic -- every page shows `Jekyll v3.10.0` vs `v4.4.1` and different timestamps
- **Missing `<script>` and `<link>` in head** (~700 diffs): seo-tag plugin generates script/link tags that rustkyll doesn't
- **Language switcher nav `<li>` elements missing** (~359 diffs): `site.data.locales | sort` iteration not populating language dropdown options
- **TOC `>` character leak** (~296 diffs): `jekyll-toc.html` include leaks a `>` character into the nav div -- likely a Liquid whitespace/escaping issue
- **URL percent-encoding of non-ASCII href** (~41 diffs): Rustkyll percent-encodes non-ASCII characters in hrefs, Jekyll outputs them as UTF-8
- **HTML entity encoding in titles** (~11 diffs): Spanish page titles show `&aacute;` instead of decoded characters
- **Smart quote differences** (existing issue 211)
- **`pquote-credit` class on `<p>` elements** (~33 diffs): Related to `markdown="1"` processing -- the `<p markdown="1" class="pquote-credit">` should have its markdown processed and the `markdown` attribute stripped

## Scope

### Fix 1: Process `markdown="1"` attribute on HTML elements

When an HTML element has `markdown="1"` as an attribute:
1. Remove the `markdown="1"` attribute from the output
2. Process the content within that element as markdown (kramdown behavior)
3. This affects `<aside markdown="1">`, `<p markdown="1">`, and `<div markdown="1">` elements

This is a kramdown feature where HTML blocks with `markdown="1"` have their content processed as markdown. See: https://kramdown.gettalong.org/syntax.html#html-blocks

Implementation notes:
- The `markdown="1"` attribute appears in the raw markdown source files (not in Liquid templates)
- The markdown processor (comrak/pulldown-cmark) likely treats these as raw HTML pass-through
- The fix should be in the kramdown postprocessing layer (`src/kramdown.rs`)
- After removing the attribute, the inner content needs to be parsed as markdown and the result inserted
- The inner content may contain block-level elements (paragraphs, images, links)

### Fix 2: Match kramdown heading ID generation for non-Latin scripts

Kramdown's default `auto_ids` generation:
1. Strips all characters not matching `[a-zA-Z0-9 -]` (NOT Unicode-aware -- only ASCII letters)
2. If the result is empty (pure non-Latin heading), uses `"section"` as the base ID
3. Appends `-1`, `-2`, etc. for duplicate IDs

Rustkyll's current `slugify()` uses `char::is_alphanumeric()` which includes Unicode alphabetic characters. This needs a configuration-aware approach:
- For the default kramdown auto_id mode: use ASCII-only letter matching
- Keep the current Unicode-preserving behavior available for sites that configure it

NOTE: This is tricky because some non-Latin scripts (Bulgarian, Greek, Bengali) DO produce non-ASCII slugs in Jekyll output. The behavior depends on whether the heading text contains any ASCII characters. Investigation needed to determine the exact kramdown algorithm. The SWE should test with actual kramdown to verify behavior before implementing.

### Fix 3: Parse `{#id}` heading ID syntax

Kramdown allows explicit heading IDs: `## Heading Text {#custom-id}`
1. Strip the `{#custom-id}` from the heading text before rendering
2. Use `custom-id` as the heading's `id` attribute (overriding auto-generated ID)
3. This is different from block IAL `{: #id}` which is already handled

## Acceptance Criteria

- [ ] `<aside markdown="1">` elements have the `markdown="1"` attribute stripped from output
- [ ] Content inside `<aside markdown="1">` is rendered as markdown (images become `<img>` inside `<p>`, text is wrapped in `<p>` tags)
- [ ] `<p markdown="1">` and `<div markdown="1">` elements are similarly processed
- [ ] Heading IDs for Arabic-only headings match kramdown output (e.g., `id="section"`, `id="section-1"`)
- [ ] Heading IDs for headings with mixed ASCII/non-ASCII content match kramdown output
- [ ] `{#custom-id}` syntax is stripped from heading text and used as the heading ID
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Tests include non-ASCII/Unicode content (Arabic, Bengali, Cyrillic headings)

## Test Scenarios

### Unit: markdown="1" attribute processing
- Parse `<aside markdown="1">\n\n![avatar](img.png)\nSome text\n\n</aside>` -- verify `markdown="1"` is stripped and content is rendered as HTML (`<img>` and `<p>` elements inside `<aside>`)
- Parse `<p markdown="1" class="pquote-credit">\n-- @user, ["Title"](url)\n</p>` -- verify `markdown="1"` is stripped, class is preserved, content is rendered as markdown
- Parse `<div markdown="1">\n## Heading\n\nParagraph\n</div>` -- verify markdown inside div is rendered
- Parse `<aside>\nRaw content\n</aside>` (no markdown attribute) -- verify content is NOT processed as markdown (pass-through behavior unchanged)

### Unit: Heading ID for non-Latin scripts (kramdown default mode)
- Arabic heading `## ما معنى` generates `id="section"` (all non-ASCII stripped, falls back to "section")
- Two Arabic headings in same document generate `id="section"` and `id="section-1"`
- Mixed heading `## GitHub هل مشاريع` generates `id="github-"` or `id="github---"` (only ASCII chars kept) -- verify exact kramdown behavior
- English heading `## Getting Started` generates `id="getting-started"` (unchanged behavior)
- Cyrillic heading `## Начало работы` -- verify against actual kramdown output (may preserve Cyrillic)

### Unit: {#id} heading ID syntax
- `## Heading Text {#custom-id}` generates `id="custom-id"` with text "Heading Text" (no `{#custom-id}` in text)
- `## تعلم قول لا {#تعلم-قول-لا}` generates `id="تعلم-قول-لا"` with correct Arabic text only
- `## No Custom ID` is unaffected (no `{#}` syntax)

### Integration: opensource-guide build
- Build opensource-guide with rustkyll and verify no Liquid errors related to markdown processing
- Verify `ar/best-practices/index.html` has `<aside>` elements without `markdown="1"` attribute
- Verify heading IDs in Arabic pages use `section`/`section-N` pattern
- Verify TOC links in Arabic pages point to correct heading IDs

## Dependencies

- Issue 196 (fix layout not applied) -- done
- No other blocking dependencies

## Implementation Notes

- Fix 1 (markdown="1") is the highest impact -- it accounts for roughly 545 of the 3474 total diffs
- Fix 2 (heading IDs) requires careful investigation of kramdown's exact algorithm; the SWE should test with `kramdown` gem directly if possible
- Fix 3 ({#id} syntax) is the simplest and most isolated fix
- All three fixes are in `src/kramdown.rs` postprocessing

## Log

- 2026-03-18: Created from cross-site comparison analysis.
- 2026-03-19: [PM] Groomed. Investigated dom-details/opensource-guide.txt (3474 total diffs). Identified 8 root causes, scoped issue to 3 fixable ones (markdown="1", heading IDs, {#id} syntax). Remaining root causes (seo-tag script/link, language switcher, TOC leak, URL encoding, HTML entities) should be tracked in separate issues.

### [SWE] 2026-03-19
- TDD cycle for Fix 3 ({#id} heading ID syntax):
  - Wrote test_heading_explicit_id_syntax, test_heading_explicit_id_arabic, test_heading_no_explicit_id_unchanged
  - Ran tests: FAIL as expected -- got id="heading-text-custom-id" with {#custom-id} in text
  - Implemented extract_explicit_heading_id() in add_heading_ids(), strips {#id} and uses it as heading ID
  - Ran tests: PASS
- TDD cycle for Fix 2 (kramdown ASCII-only heading IDs):
  - Wrote test_slugify_arabic_only_falls_back_to_section, test_slugify_two_arabic_headings_unique_ids, test_slugify_mixed_ascii_arabic, test_slugify_english_unchanged
  - Ran tests: FAIL as expected -- got Arabic text in slug instead of "section"
  - Reimplemented slugify() to match kramdown's exact algorithm: strip leading non-ASCII-alpha, keep only [a-zA-Z0-9 -], downcase, fall back to "section"
  - Updated 8 existing Cyrillic/leading-digit tests that assumed incorrect Unicode-preserving behavior
  - Ran tests: PASS (20 slugify tests, 2 heading ID tests)
- TDD cycle for Fix 1 (markdown="1" attribute processing):
  - Wrote test_process_markdown_attr_aside, _p_with_class, _div, _absent
  - Ran tests: FAIL as expected -- stub returned content unchanged
  - Implemented process_markdown_attribute(): finds HTML blocks with markdown="1", strips the attribute, renders inner content as markdown using pulldown-cmark, handles block vs inline containers
  - Integrated into markdown_to_html() and markdown_to_html_with_options() as preprocessing step
  - Ran tests: PASS (4 markdown attr tests)
- Full suite: 1873 lib tests + all integration tests pass, 0 failures
- Clippy: clean (no warnings)
- Fmt: clean
- Files modified: src/kramdown.rs, src/frontmatter.rs
- Files status: docs/tracker/228 renamed to .in-progress.md

### [SWE] 2026-03-19 (QA fix: UTF-8 panic in find_markdown_close_tag)
- Bug: find_markdown_close_tag uses `pos += 1` to advance byte-by-byte through HTML content. Multi-byte UTF-8 characters (Chinese, Japanese, Korean, Arabic) cause panic at string slice boundary.
- TDD cycle:
  - Wrote test_process_markdown_attr_cjk_content (Chinese/Japanese/Korean inside aside markdown="1")
  - Wrote test_process_markdown_attr_arabic_content (Arabic inside aside markdown="1")
  - Wrote test_find_markdown_close_tag_with_multibyte_utf8 (direct unit test)
  - Ran tests: PANIC as expected -- "byte index 3 is not a char boundary; it is inside '这' (bytes 2..5)"
  - Fixed: replaced `pos += 1` with `pos += html[pos..].chars().next().map_or(1, |c| c.len_utf8())` in find_markdown_close_tag (line 326)
  - Checked other `pos += 1` loops in kramdown.rs: line 894 iterates over line indices (safe), line 1033 iterates over ASCII bytes only (safe)
  - Ran tests: PASS (all 3 new tests pass)
- Full suite: 1876 lib tests pass, 0 failures
- Clippy: clean
- Fmt: clean
- Recount DOM: opensource-guide 23/388 matches (up from 0/388)
- Files modified: src/kramdown.rs
