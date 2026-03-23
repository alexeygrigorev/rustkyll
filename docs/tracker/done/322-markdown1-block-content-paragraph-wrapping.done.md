# Issue 322: `markdown="1"` block content not wrapping inline content in `<p>` tags

## Problem

When kramdown processes a block element with `markdown="1"` (e.g.,
`<aside markdown="1">`), the inner content is re-parsed as block-level
markdown. This means loose inline content (text, `<img>` tags, etc.) gets
wrapped in `<p>` elements, just like any other markdown block content would be.

Rustkyll's `process_markdown_attribute()` in `src/kramdown.rs` uses
pulldown-cmark to render the inner content. However, pulldown-cmark treats
`<img>` tags at the start of a line as beginning an HTML block, which
suppresses paragraph wrapping of the surrounding text. This causes a structural
difference from kramdown's output.

### Concrete example

Source markdown (used extensively in opensource-guide):

```html
<aside markdown="1" class="pquote">
  <img src="https://avatars.githubusercontent.com/lord?s=180" class="pquote-avatar" alt="avatar">
  I fumbled it. I didn't put in the effort to come up with a complete solution.
  <p markdown="1" class="pquote-credit">
  -- @lord, ["Tips for new open source maintainers"](https://lord.io/blog/2014/oss-tips/)
  </p>
</aside>
```

**Jekyll (correct) output:**
```html
<aside class="pquote">
  <p><img src="https://avatars.githubusercontent.com/lord?s=180" class="pquote-avatar" alt="avatar" />
  I fumbled it. I didn't put in the effort to come up with a complete solution.</p>
  <p class="pquote-credit">
  -- @lord, <a href="https://lord.io/blog/2014/oss-tips/">"Tips for new open source maintainers"</a>
  </p>
</aside>
```

**Rustkyll (current) output:**
```html
<aside class="pquote">
<img src="https://avatars.githubusercontent.com/lord?s=180" class="pquote-avatar" alt="avatar" />
  I fumbled it. I didn't put in the effort to come up with a complete solution.
  <p class="pquote-credit">-- @lord, <a href="https://lord.io/blog/2014/oss-tips/">"Tips for new open source maintainers"</a></p>
</aside>
```

Key differences:
1. The `<img>` and following text are NOT wrapped in a `<p>` tag in rustkyll
2. The `<img>` should be inside a `<p>` alongside the adjacent text

### Impact

This affects **316 pages** in opensource-guide (all 13 articles x ~27 language
translations, minus the 23 that already match). Each page has 5-15 `<aside>`
blocks with this pattern, producing 6000+ individual DOM diffs.

The aside/pquote pattern is the single biggest remaining diff category for
opensource-guide (currently 23/388, target 300+/388 after this fix).

This may also affect other sites that use `markdown="1"` on block elements with
inline HTML content inside, though opensource-guide is the primary case.

### Root cause

In `src/kramdown.rs`, `process_markdown_attribute()` (line 703+) extracts the
inner content of `<aside markdown="1">`, trims it, and passes it to
pulldown-cmark. Pulldown-cmark follows CommonMark HTML block rules (type 6):
when a line starts with an HTML block-level tag or a self-closing tag like
`<img`, it enters an HTML block context. This prevents the text from being
wrapped in `<p>` tags.

Kramdown behaves differently: it treats content inside `markdown="1"` blocks
as fresh markdown input where `<img>` is an inline element, so the `<img>` and
adjacent text get wrapped in a `<p>`.

## Scope

### In scope

1. **Fix paragraph wrapping of inline HTML inside `markdown="1"` blocks** --
   ensure that when `process_markdown_attribute()` processes block-level content,
   inline HTML elements like `<img>` followed by text are wrapped in `<p>` tags,
   matching kramdown's behavior.

2. **Handle the specific pattern**: `<img ...>` on its own line followed by text
   should be treated as inline content (not an HTML block), resulting in a `<p>`
   wrapping both the image and the text.

3. **Preserve existing behavior** for nested `<p markdown="1">` (which already
   works after issue 320) and for `<div markdown="1">` blocks that contain only
   text/markdown content (no inline HTML).

### Possible approaches

- **Pre-processing**: Before sending inner content to pulldown-cmark, detect
  standalone `<img>` lines and join them with the following text line so
  pulldown-cmark sees them as inline content within a paragraph.
- **Post-processing**: After pulldown-cmark rendering, detect unwrapped inline
  elements (`<img>`, text) that are direct children of the markdown="1" block
  and wrap them in `<p>` tags.
- **Custom parsing**: For `markdown="1"` blocks specifically, use kramdown-like
  logic that treats all non-block-level HTML as inline content.

The SWE should choose the approach that best matches kramdown's actual behavior
and has the least risk of regressions.

### Out of scope

- Other opensource-guide diffs (head meta/script ordering, nav `<li>` missing,
  extra `>` text in nav -- these are template/include issues)
- Changes to how normal markdown (outside `markdown="1"` blocks) handles HTML
- Processing of `markdown="block"` or `markdown="span"` variants (if already
  working correctly)

## Dependencies

- Issue 320 (done): Nested `markdown="1"` attribute stripping and
  `basic_generate_id` for headings. This issue builds on 320's recursive
  processing.

## Key Files to Modify

- `src/kramdown.rs` -- `process_markdown_attribute()` function (line 703+),
  specifically the section that prepares inner content before passing to
  pulldown-cmark, or the section that processes pulldown-cmark's output

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] Content inside `<aside markdown="1">` with `<img>` + text produces a
      `<p>` wrapping the `<img>` and text together
- [ ] `<p markdown="1" class="pquote-credit">` inside `<aside>` continues to
      render correctly with inline markdown processed and class preserved
- [ ] `<div markdown="1">` with only text content (no inline HTML) still
      renders correctly with `<p>` wrapping
- [ ] `<div markdown="1">` with heading + paragraphs still renders correctly
- [ ] opensource-guide DOM comparison improves from 23/388 to 200+/388
- [ ] opensource-guide `best-practices/index.html`: all `<aside>` blocks have
      `<p>` wrapping `<img>` + text content
- [ ] No regressions: DTC remains 746+/790, muan-blog remains 2172+/2218
- [ ] All sites currently at 100% remain at 100%
- [ ] Tests include non-ASCII content (aside with Unicode text/CJK characters)

## Test Scenarios

### Unit: `<img>` + text wrapped in `<p>` inside `markdown="1"` block

- Input:
  ```html
  <aside markdown="1" class="pquote">
    <img src="test.jpg" class="avatar" alt="avatar">
    Some text about open source.
    <p markdown="1" class="credit">
    -- @user, ["Article"](https://example.com)
    </p>
  </aside>
  ```
  Expected output:
  ```html
  <aside class="pquote">
    <p><img src="test.jpg" class="avatar" alt="avatar" />
    Some text about open source.</p>
    <p class="credit">
    -- @user, <a href="https://example.com">"Article"</a>
    </p>
  </aside>
  ```

### Unit: `<div markdown="1">` with only text (no inline HTML)

- Input: `<div markdown="1">\nSome text.\n</div>`
  Expected: `<div>\n<p>Some text.</p>\n</div>` (no change from current behavior)

### Unit: `<div markdown="1">` with headings and paragraphs

- Input: `<div markdown="1">\n## Title\n\nParagraph text.\n</div>`
  Expected: `<div>\n<h2 ...>Title</h2>\n<p>Paragraph text.</p>\n</div>`
  (no change from current behavior)

### Unit: multiple `<img>` elements followed by text

- Input:
  ```html
  <div markdown="1">
    <img src="a.jpg" alt="a">
    <img src="b.jpg" alt="b">
    Text after images.
  </div>
  ```
  Expected: both images and text wrapped in `<p>` (matching kramdown behavior)

### Unit: Unicode content inside `markdown="1"` block

- Input:
  ```html
  <aside markdown="1" class="pquote">
    <img src="avatar.jpg" alt="avatar">
    Contribuer au logiciel libre, c'est important.
    <p markdown="1" class="credit">
    -- @utilisateur, ["Guide du debutant"](https://example.com)
    </p>
  </aside>
  ```
  Expected: French text and `<img>` wrapped in `<p>`, accented characters
  preserved

### Integration: opensource-guide site build

- Build opensource-guide with rustkyll
- Verify `best-practices/index.html` has `<p>` wrapping `<img>` + text inside
  all `<aside>` blocks
- Verify `ar/best-practices/index.html` has same structure (Arabic content)
- Run DOM comparison, verify 200+/388 pages match (up from 23)

### Integration: Regression check

- Build DTC, verify no regression (746+/790)
- Build muan-blog, verify no regression (2172+/2218)
- Build all 100% sites, verify they remain at 100%
- Run `cargo test` full suite

## Output Verification

```bash
./scripts/cargo-safe build --release

# opensource-guide
./target/release/rustkyll build \
  --source websites/opensource-guide/ \
  --destination /tmp/osg_322

# Verify aside blocks have <p> wrapping img + text
# In Jekyll output, the pattern is: <aside ...>\n  <p><img ... />\n  text</p>
grep -A3 '<aside class="pquote">' /tmp/osg_322/best-practices/index.html | head -12
# Must show <p><img ...> pattern, NOT bare <img>

# Count aside blocks missing <p> wrapping
python3 -c "
import re
html = open('/tmp/osg_322/best-practices/index.html').read()
# Find aside blocks where img is direct child (no <p> wrapper)
bad = len(re.findall(r'<aside[^>]*>\s*\n\s*<img', html))
good = len(re.findall(r'<aside[^>]*>\s*\n\s*<p><img', html))
print(f'Wrapped: {good}, Unwrapped: {bad}')
"
# Target: 0 unwrapped

# DOM comparison
uv run scripts/dom_compare.py \
  --jekyll-dir websites/opensource-guide/_site_jekyll_cached \
  --rustkyll-dir /tmp/osg_322
# Target: 200+/388 matched (up from 23)

# Regression checks
./target/release/rustkyll build \
  --source websites/DataTalksClub/datatalksclub.github.io \
  --destination /tmp/dtc_322
uv run scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_322
# Must remain 746+/790

./target/release/rustkyll build \
  --source websites/muan-blog \
  --destination /tmp/muan_322
uv run scripts/dom_compare.py \
  --jekyll-dir websites/muan-blog/_site_jekyll_cached \
  --rustkyll-dir /tmp/muan_322
# Must remain 2172+/2218
```

## Log

### [SWE] 2026-03-23

**TDD Cycle:**

1. Wrote 7 failing tests in src/kramdown.rs:
   - `test_process_markdown_attr_img_plus_text_wrapped_in_p` - img+text wrapped in `<p>`
   - `test_process_markdown_attr_img_text_with_nested_p_markdown` - full aside pattern with nested `<p markdown="1">`
   - `test_process_markdown_attr_multiple_img_plus_text` - multiple `<img>` elements + text
   - `test_process_markdown_attr_img_text_unicode` - French accented text with img
   - `test_process_markdown_attr_img_text_cjk` - Japanese CJK text with img
   - `test_process_markdown_attr_div_text_only_no_regression` - regression: div with text only
   - `test_process_markdown_attr_div_heading_paragraph_no_regression` - regression: div with heading+paragraph

2. Ran tests: 4 FAILED as expected (img+text tests), 2 regression tests PASSED (existing behavior ok), 1 multiple-img FAILED

3. Implemented fix: added `preprocess_inline_html_for_markdown()` function in src/kramdown.rs
   - Root cause: pulldown-cmark treats `<img` at start of line as CommonMark HTML block type 6, suppressing `<p>` wrapping
   - Fix: pre-processes inner content of `markdown="1"` blocks by joining `<img>` lines with following text lines using space, so pulldown-cmark sees inline content (triggers paragraph mode)
   - Integrated into `process_markdown_attribute()` before pulldown-cmark parsing

4. Ran tests: all 7 new tests PASS, all 6 existing markdown attr tests PASS (13 total)
5. Full test suite: 2882+ tests pass, 0 failures
6. Clippy clean, fmt clean

**Files modified:**
- `src/kramdown.rs` - added `preprocess_inline_html_for_markdown()` helper function, integrated into `process_markdown_attribute()`, added 7 tests
