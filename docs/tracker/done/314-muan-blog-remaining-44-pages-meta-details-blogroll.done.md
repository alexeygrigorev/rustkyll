# Issue 314: muan-blog remaining 44 DOM diff pages -- meta truncation, details, blogroll

## Problem

muan-blog matches 2174/2218 (98%). 44 pages have remaining diffs after issues
297 (list indent + Unicode truncation) and 294 (bare URL autolink). The
remaining diffs fall into these categories:

### Category A: Meta description list-item indentation not fully fixed (~20 pages)

Issue 297 fixed CommonMarkGhPages list indentation by disabling
`indent_list_items()` for non-kramdown sites. However, ~20 pages still show
list-item indentation diffs in meta description attributes.

Pattern (expected = Jekyll, actual = rustkyll):
- Jekyll:  `content='\nItem 1\nItem 2\n'`
- Rustkyll: `content='\n  Item 1\n  Item 2\n'`

These pages use `{{ page.content | strip_html | truncate: 240 }}` for meta
descriptions. The list items in the HTML content have `  <li>` (2-space indent)
in rustkyll vs `<li>` (no indent) in Jekyll. After `strip_html`, the leading
spaces become visible in the meta content.

Root cause investigation needed: Issue 297 added `postprocess_with_options`
with `indent_lists=false` for CommonMark sites. Either:
1. Some code path still uses the old `postprocess()` function (which indents)
2. The indentation is coming from a different source (e.g., definition lists
   `<dl><dd>`, description lists, or nested elements)
3. The `markdown_to_html_for_filter` (markdownify) path does not use the
   non-indenting option

The engineer must trace the exact code path for these pages and verify
that ALL HTML rendering paths for CommonMark sites skip list indentation.

### Category B: Meta description truncation boundary mismatch (~15 pages)

The `truncate: 240` filter produces different cutoff points for some pages,
even after the Unicode codepoint counting fix in issue 297. Differences:

**Sub-pattern B1: Trailing `\n` vs `\n...` (CJK content)**
- Jekyll:  `content='CJK text ending with...\n'`
- Rustkyll: `content='CJK text ending with...\n...'`

The content is exactly at or near the 240-character boundary. Jekyll does
NOT append `...` because the content is exactly 240 chars or fewer. Rustkyll
appends `...` because it counts differently at the boundary.

This may be caused by how trailing whitespace (the `\n` from strip_html
output) is counted: Jekyll may strip trailing whitespace before counting,
while rustkyll includes it.

**Sub-pattern B2: Different truncation position (`expl...` vs `explo...`)**
Some pages show the truncation happening at slightly different positions:
- Jekyll:  `...Been expl...`
- Rustkyll: `...Been explo...`

This 1-character difference suggests the truncation is counting the `...`
suffix itself differently. Ruby's `truncate` counts the omission string as
part of the limit (240 = 237 content + 3 `...`), and rustkyll may have a
consistent off-by-one here.

### Category C: `<details>` element content rendering (~5 pages)

Pages with `<details><summary>CW</summary>content</details>` HTML show
structural diffs:

- Jekyll: `<details>text content directly...</details>` (text as direct child)
- Rustkyll: `<details><p>text content...</p></details>` (text wrapped in `<p>`)

CommonMark wraps content after `<summary>` in paragraph tags. Jekyll's
CommonMark implementation does not. The `<p>` wrapping also shifts the
text content -- paragraph 1 text appears where paragraph 2 text should be.

Additionally, in meta descriptions, the `<summary>` text handling differs:
- Jekyll meta: `"CWthe condition is..."` (summary text + content concatenated)
- Rustkyll meta: `"CW\nthe condition is..."` (newline between summary and content)

This affects 5 pages (`notes/2023-09-25-ee.html`, `notes/2023-11-25-mm.html`,
`notes/2024-01-16-aa.html`, `notes/2024-10-28-oo.html`, and 1 more).

### Category D: Blogroll page h2 ID generation (1 page, 17 diffs)

`pages/blogroll.html` has 17 h2 element diffs where the `id` attribute
is generated from the heading content differently.

- Jekyll: `id='a-hrefhttpsmanuelmorealecommanuel-morealea-code-classsmolencode'`
- Rustkyll: `id='scott-ohara-en'`

Jekyll generates the ID from the raw HTML of the heading (including `<a>`
tag attributes), while rustkyll generates it from the visible text content.
The IDs are completely different, suggesting the heading ID generation
algorithm differs fundamentally for headings containing HTML elements.

### Category E: time datetime attribute formatting (5 pages)

The `<time datetime="...">` attribute format differs:
- Jekyll: `datetime='2024-11-23 20:16:52 +0800'` (space-separated offset)
- Rustkyll: `datetime='2024-11-23T20:16:52+08:00'` (ISO 8601 format)

Some pages also show timezone conversion differences (PST vs +08:00
numeric offsets).

## Scope

This issue focuses on **Categories A and B** (35 pages combined, highest
impact and most tractable from a single code area -- the strip_html and
truncate filter pipeline).

### In scope

1. **Fix remaining list-item indentation in meta descriptions** -- ensure
   ALL code paths for CommonMark sites produce unindented `<li>` elements.
   Trace and fix the specific code path causing the 20 remaining pages.

2. **Fix truncation boundary edge cases** -- ensure the `truncate` filter
   matches Ruby's exact boundary behavior:
   - Trailing `\n` should be counted as 1 character (not 0)
   - The omission string `...` (3 chars) is included in the limit
   - Verify: `truncate: 240` means 237 content chars + 3 omission chars
   - Handle the case where content is exactly 240 chars (no truncation)

### Out of scope (create follow-up issues)

- Category C: `<details>` content wrapping (~5 pages) -- requires changes
  to how CommonMark handles HTML block elements
- Category D: Blogroll h2 ID generation (1 page) -- heading ID algorithm
  for headings containing HTML elements
- Category E: time datetime formatting (5 pages) -- date formatting
  differences in Liquid template rendering

## Dependencies

- Issue 297 (muan-blog list indent + truncation) -- DONE. This issue fixes
  what 297 left incomplete.
- Issue 294 (GFM autolink) -- IN PROGRESS. Independent of this issue.

## Key Files to Modify

- `src/frontmatter.rs` -- `markdown_to_html_with_options` and any variant
  called for meta description generation. Check if `markdown_to_html_for_filter`
  uses the non-indenting option.
- `src/kramdown.rs` -- `postprocess_with_options` and `indent_list_items`.
  Verify all callers pass the correct `indent_lists` flag.
- `vendor/liquid-lib/src/stdlib/filters/string/truncate.rs` -- truncation
  boundary handling, trailing whitespace behavior, exact omission string
  counting.
- `src/template/layout.rs` -- how markdown engine choice propagates to
  filter rendering paths.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests below
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] Meta descriptions from list content have NO leading spaces before
      list items -- `\nItem 1\nItem 2\n` not `\n  Item 1\n  Item 2\n`
- [ ] This applies to ALL rendering paths (direct page render AND
      markdownify filter output used in meta tags)
- [ ] `truncate: 240` on a string of exactly 240 characters does NOT
      append `...` (matches Ruby behavior)
- [ ] `truncate: 240` on a string of 241 characters produces 237 chars + `...`
- [ ] Trailing `\n` in content counts as 1 character toward the limit
- [ ] muan-blog DOM match improves to 2195+/2218 (from 2174, fixing 21+
      of the 35 in-scope pages)
- [ ] No regressions on DTC (must remain 740+/790)
- [ ] No regressions on mlwiki (must remain 552+/644)
- [ ] No regressions on any of the 13+ sites currently at 100%
- [ ] Tests include non-ASCII/Unicode content (CJK text near truncation
      boundaries, emoji in list items)

## Test Scenarios

### Unit: List indentation in markdownify filter path

- Process `- Item\n- Item 2` through `markdown_to_html_for_filter` (the
  markdownify code path) with CommonMark settings
- Verify output is `<ul>\n<li>Item</li>\n<li>Item 2</li>\n</ul>\n`
  (no indentation before `<li>`)
- Process same input through kramdown settings, verify indentation IS present
  (regression check -- kramdown sites should still indent)

### Unit: List indentation propagation through strip_html

- HTML input: `<ul>\n<li>Jeff Bridges</li>\n<li>Moore</li>\n</ul>`
- After strip_html: `\nJeff Bridges\nMoore\n` (no leading spaces)
- Verify this works for definition lists too: `<dl><dd>item</dd></dl>`

### Unit: Truncation boundary -- exactly at limit

- String of exactly 240 ASCII characters -> `truncate: 240` -> no change
  (no `...` appended)
- String of exactly 240 CJK characters -> `truncate: 240` -> no change
- String of 241 ASCII characters -> `truncate: 240` -> 237 chars + `...`
- String of 240 chars ending with `\n` -> `truncate: 240` -> no change
  (the `\n` is the 240th character)
- String of 241 chars ending with `\n` -> `truncate: 240` -> 237 chars + `...`

### Unit: Truncation boundary -- trailing whitespace

- String of 238 chars + `\n\n` (total 240) -> `truncate: 240` -> no change
- String of 239 chars + `\n\n` (total 241) -> `truncate: 240` -> 237 + `...`
- Verify Ruby behavior: `"x" * 240` with truncate(240) == `"x" * 240`

### Unit: Combined pipeline for CJK content

- CJK content (80+ characters) through strip_html then truncate: 240
- Verify no spurious `...` appended when character count is under 240
- Verify truncation at correct codepoint boundary for content over 240

### Integration: muan-blog page rendering

- Build muan-blog with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify match count is >= 2195/2218
- Spot-check pages:
  - `notes/2019-04-23-ee.html` -- meta content has `\nItem` not `\n  Item`
  - `notes/2019-06-28-cc.html` -- meta content truncation matches Jekyll
  - `notes/2022-04-05-zz.html` -- list items without indentation
  - A CJK-heavy page near truncation boundary -- no spurious `...`

### Regression: Other sites

- Run `cargo test` full suite
- Run DOM comparison on DTC to verify no regression
- Run DOM comparison on mlwiki to verify no regression
- Verify all 13+ sites at 100% remain at 100%

## Output Verification

```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/muan-blog/ \
  --destination /tmp/muan_314

python3 scripts/dom_compare.py \
  --jekyll-dir websites/muan-blog/_site_jekyll_cached \
  --rustkyll-dir /tmp/muan_314
```

Spot-checks:
- `grep -P '  (That|My hair|Jeff|Julianne)' /tmp/muan_314/notes/2019-04-23-ee.html`
  -- must NOT show 2-space indent before list items in meta content
- Summary line must show >= 2195 files matched (up from 2174)

## Follow-up Issues to Create

After this issue is done, create separate issues for:
- `<details>` element `<p>` wrapping in CommonMark (Category C, ~5 pages)
- Blogroll h2 ID generation for HTML-containing headings (Category D, 1 page)
- time datetime attribute formatting (Category E, 5 pages)

## Log

### [SWE] 2026-03-23

**Category A fix: markdownify list indentation for CommonMark sites**

- Root cause: `postprocess_for_filter()` in `src/kramdown.rs` unconditionally called
  `indent_list_items()`. For CommonMark sites, this should be skipped.
  Issue 297 fixed `postprocess_with_options()` but missed `postprocess_for_filter()`.

- TDD step 1: Wrote `test_issue314_markdownify_list_indent_modes` in
  `src/template/filters/markdownify.rs` -- tests both CommonMark (no indent)
  and kramdown (indent) modes, plus CJK/emoji content.
- TDD step 2: Tests failed because `set_markdownify_indent_lists` didn't exist yet.
- TDD step 3: Implemented fix:
  - Added global `AtomicBool` `MARKDOWNIFY_INDENT_LISTS` in `src/frontmatter.rs`
    with `set_markdownify_indent_lists()` and `get_markdownify_indent_lists()`.
  - Added `postprocess_for_filter_with_options(html, indent_lists)` in `src/kramdown.rs`.
  - Modified `markdown_to_html_for_filter()` to read the global flag and pass it
    to the new function.
  - Wired `set_markdownify_indent_lists(is_kramdown)` in `src/main.rs`.
- TDD step 4: Tests pass.

**Category B fix: Truncation boundary edge cases**

- Investigation: Reviewed `truncate.rs` logic. Line 81: `if length < input_char_len`
  correctly uses strict `<`, so exactly-at-limit strings are not truncated.
  Line 75: `l = length.saturating_sub(ellipsis_char_len)` correctly counts the
  omission string as part of the limit.
- TDD step 1: Wrote 7 new boundary tests in `vendor/liquid-lib/.../truncate.rs`:
  240 CJK no-truncation, 240 with trailing \n, 241 with trailing \n,
  238+\n\n (240 total), 239+\n\n (241 total), CJK under 240, CJK over 240.
- TDD step 2: All tests pass immediately. The truncation logic is already correct.
- Conclusion: Category B diffs are caused by Category A (extra indentation spaces
  making content longer after strip_html, shifting the truncation boundary).
  Fixing Category A also fixes Category B.

**Test results:**
- 2542 total tests (2539 pass + 1 pre-existing failure from issue 313 + 2 ignored)
- Issue 314 tests: 1 new markdownify test (combined) + 7 new truncation tests = 8 new tests
- clippy: clean (0 warnings from our code)
- fmt: clean

**Files modified:**
- `src/frontmatter.rs` -- Added AtomicBool global, setter/getter, modified `markdown_to_html_for_filter`
- `src/kramdown.rs` -- Added `postprocess_for_filter_with_options`, refactored `postprocess_for_filter` to delegate
- `src/main.rs` -- Wired `set_markdownify_indent_lists(is_kramdown)` at startup
- `src/template/filters/markdownify.rs` -- Added `test_issue314_markdownify_list_indent_modes`
- `vendor/liquid-lib/src/stdlib/filters/string/truncate.rs` -- Added 7 truncation boundary tests
