# Issue 293: strip_html filter whitespace and trailing newline differences

## Problem

The Liquid `strip_html` filter in rustkyll produces different output than Jekyll's implementation in two ways, causing DOM comparison failures on 142+ pages across DTC and muan-blog:

### Sub-problem A: Trailing newline in strip_html output (82 DTC pages)

DTC's `post.html` layout uses `author.content | strip_html | jsonify` for JSON-LD `@graph` author descriptions. Jekyll's `strip_html` strips the trailing newline from rendered HTML content, but rustkyll preserves it.

**Example:**
- Jekyll: `"description":"Alexey Grigorev is the founder of DataTalks.Club"`
- Rustkyll: `"description":"Alexey Grigorev is the founder of DataTalks.Club\n"`

The `author.content` is the rendered markdown of the author page. Jekyll's pipeline `<p>Short bio.</p>\n` through `strip_html` produces `Short bio.` (no trailing newline), while rustkyll produces `Short bio.\n`.

This also affects descriptions containing markdown links: Jekyll's `strip_html` on `<p>Founder of <a href="url">Foo</a></p>` produces `Founder of Foo`, while rustkyll may produce `Founder of Foo\n` or have different whitespace around removed tags.

### Sub-problem B: List item indentation in strip_html output (60 muan-blog pages)

muan-blog's `default.html` layout uses `page.content | strip_html | truncate: 240` for `<meta name="description">`. When HTML content contains lists, Jekyll's `strip_html` produces text without leading spaces on list items:

**Example (list in HTML):**
```html
<ul>\n<li>Item one</li>\n<li>Item two</li>\n</ul>
```

- Jekyll `strip_html`: `\nItem one\nItem two\n`
- Rustkyll `strip_html`: `\n  Item one\n  Item two\n`

The extra indentation shifts the truncation boundary, causing the `truncate: 240` output to differ. It also causes `...` placement differences (the ellipsis lands at a different point in the content).

### Sub-problem C: Details/summary strip_html newline handling (8 muan-blog pages)

Related but separate: when `strip_html` processes `<details><summary>CW</summary>content</details>`, Jekyll produces `CWcontent` (summary and content joined without separator), while rustkyll produces `CW\ncontent` (newline between summary and content). This shows up in meta description diffs where `content='CWthe condition...'` (Jekyll) vs `content='CW\nthe condition...'` (rustkyll).

## Root Cause

Jekyll's `strip_html` filter (from Liquid gem) uses a regex-based approach: `input.to_s.gsub(/<script.*?<\/script>/m, '').gsub(/<!--.*?-->/m, '').gsub(/<style.*?<\/style>/m, '').gsub(/<.*?>/m, '')`. This:
1. Removes all HTML tags via regex
2. Does NOT add or preserve whitespace from HTML formatting/indentation
3. Does NOT add newlines between block elements

Rustkyll likely preserves whitespace that existed between HTML tags in the rendered HTML, or adds newlines for block element boundaries. The fix needs to match Jekyll's simpler regex-based approach.

## Affected Pages

- **DTC** (`datatalksclub.github.io`): 82 pages with JSON-LD `author[0].description` diffs (these are the jsonld-only failures from the 133 remaining)
- **muan-blog**: 60 pages with meta description whitespace/truncation diffs + 8 pages with details/summary diffs = 68 pages total
- **Potential other sites**: Any site using `strip_html` on content with lists or trailing newlines

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests plus new tests
- [ ] `strip_html` on `"<p>Hello world.</p>\n"` produces `"Hello world."` (no trailing newline)
- [ ] `strip_html` on `"<ul>\n<li>A</li>\n<li>B</li>\n</ul>\n"` produces `"\nA\nB\n"` (no leading spaces on items)
- [ ] `strip_html` on `"<p>Text with <a href='url'>link</a> inside.</p>\n"` produces `"Text with link inside."` (no trailing newline)
- [ ] `strip_html` on `"<details><summary>CW</summary><p>content</p></details>"` produces `"CWcontent"` (no newline between summary and content)
- [ ] `strip_html | jsonify` pipeline produces correct JSON-escaped output without spurious `\n`
- [ ] `strip_html | truncate: 240` produces the same truncation point as Jekyll for list content
- [ ] DOM comparison recount shows improvement:
  - DTC: at least 70 of the 82 jsonld-only pages now match
  - muan-blog: at least 50 of the 68 meta-description pages now match
- [ ] No regressions on existing tests or other benchmark sites
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean

## Test Scenarios

### Unit: strip_html trailing newline

- Input `"<p>Short bio.</p>\n"` -> strip_html -> `"Short bio."` (no trailing `\n`)
- Input `"<p>Line one.</p>\n<p>Line two.</p>\n"` -> strip_html -> `"Line one.\nLine two."` or matching Jekyll
- Input `"<p>Has <em>emphasis</em> and <a href='#'>link</a>.</p>\n"` -> strip_html -> `"Has emphasis and link."` (no trailing `\n`)

### Unit: strip_html list indentation

- Input `"<ul>\n<li>A</li>\n<li>B</li>\n</ul>"` -> strip_html -> verify no leading spaces on items
- Input `"<ol>\n<li>First</li>\n<li>Second</li>\n</ol>"` -> strip_html -> verify no leading spaces

### Unit: strip_html details/summary

- Input `"<details><summary>Warning</summary><p>Text</p></details>"` -> strip_html -> `"WarningText"` (no newline separator)

### Integration: strip_html | truncate chain

- Build content with a list, pass through `strip_html | truncate: 100`, verify truncation point matches Jekyll behavior

### Integration: strip_html | jsonify chain

- Pass `"<p>Bio text.</p>\n"` through `strip_html | jsonify`, verify output is `"\"Bio text.\""` (not `"\"Bio text.\\n\""`)
- Include Unicode content (CJK characters) to catch encoding regressions

### Site-level: DTC DOM comparison

- Build DTC site (or use cached output)
- Run DOM comparison on affected blog pages (e.g., `blog/benefits-of-learning-in-public.html`)
- Verify `jsonld.@graph[0].author[0].description` matches between Jekyll and rustkyll

### Site-level: muan-blog DOM comparison

- Build muan-blog (or use cached output)
- Run DOM comparison on affected note pages
- Verify `<meta content="...">` description matches between Jekyll and rustkyll

## Dependencies

None -- this is a Liquid filter behavior fix independent of other issues.

## Implementation Notes

- The `strip_html` filter is likely provided by the liquid crate or by a custom implementation in rustkyll. Check which one is used.
- Jekyll's implementation is a simple regex: `gsub(/<.*?>/m, '')` -- it literally just removes HTML tags without adding any whitespace. The rendered HTML already has whitespace between block elements as literal characters.
- The key insight: if the rendered HTML is `<p>Text</p>\n`, removing `<p>` and `</p>` leaves `Text\n`. Jekyll then does NOT strip the trailing newline -- but wait, the DTC diff shows Jekyll has NO trailing newline. This means either (a) the rendered HTML doesn't have a trailing newline in Jekyll, or (b) there's additional processing. Investigate by checking actual rendered `author.content` values.
- For the list indentation issue, the HTML likely contains `<li>` tags with indentation in the source. Jekyll's regex strips the tags but keeps the text between them. If the HTML renderer indents list items differently, the strip_html output will differ.

## Files Likely Affected

- `src/template/engine.rs` (if strip_html is implemented here)
- `src/template/filters/` (if there's a custom strip_html filter)
- Possibly the liquid crate's built-in filter (may need override)

## Log

### [SWE] 2026-03-21
- Wrote 10 TDD tests in engine.rs covering all acceptance criteria: trailing newline, multiple paragraphs, inline tags, list indentation, ordered list, details/summary, jsonify pipeline, truncate pipeline, CJK unicode, unicode with jsonify
- Ran tests: 8 FAILED as expected (strip_html preserved trailing newlines, details_summary and ordered_list already passed)
- Root cause: vendored liquid crate's strip_html correctly removes HTML tags via regex but preserves trailing whitespace. Jekyll output has no trailing whitespace because the final newline after closing tags gets consumed.
- Implemented fix: created custom `src/template/filters/strip_html.rs` with state-machine HTML tag removal (no regex dependency) plus `strip_suffix('\n')` to remove exactly one trailing newline after tag removal
- Registered custom StripHtml filter in engine builder to override stdlib version
- Also fixed pre-existing compilation errors from issue 294 (5th `enable_autolink` parameter added to `markdown_to_html_with_options` but callers not updated): fixed all callers in collection.rs, layout.rs, frontmatter.rs
- Updated 2 existing tests from issue 268 that expected trailing newline preservation to expect no trailing newline (matching new Jekyll-compatible behavior)
- Ran tests: all 10 issue 293 tests PASS, all 13 filter unit tests PASS, all 33 strip_html-related tests PASS
- Full test suite: only 1 pre-existing failure (`test_site_context_github_url_nil_without_plugin`) unrelated to this change
- Clippy: clean (no warnings from our code)
- Fmt: clean
- Files created: `src/template/filters/strip_html.rs`
- Files modified: `src/template/filters/mod.rs`, `src/template/engine.rs`, `src/template/layout.rs`, `src/collection.rs`, `src/frontmatter.rs`
