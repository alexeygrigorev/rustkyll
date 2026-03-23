# Issue 320: Fix `markdown="1"` on nested HTML elements and heading IDs inside markdown blocks

## Problem

opensource-guide matches only 23/388 pages (6%). Two related issues account for
the vast majority of diffs on all 365 failing pages:

### 1. `markdown="1"` not stripped from nested `<p>` elements

When a `<div markdown="1">` or `<aside markdown="1">` block contains nested
HTML elements like `<p markdown="1">`, the attribute on the inner `<p>` is not
being stripped and the content is not processed as markdown.

Source content pattern (used extensively in opensource-guide):
```html
<aside markdown="1" class="pquote">
  <img src="..." class="pquote-avatar" alt="...">
  Quoted text here...
  <p markdown="1" class="pquote-credit">
    -- @username, ["Article Title"](url)
  </p>
</aside>
```

**Current behavior**: `<aside>` attribute is correctly stripped, but `<p markdown="1">`
appears verbatim in output with the attribute still present.

**Expected behavior**: Both `<aside>` and `<p>` should have `markdown="1"`
stripped, and content inside `<p>` should be processed as inline markdown (links,
emphasis, etc. rendered to HTML).

### 2. Heading IDs inside `markdown="1"` blocks use wrong algorithm

In kramdown, content inside `markdown="1"` blocks is re-parsed by the **base
kramdown parser** (not the GFM parser). This means heading IDs use kramdown's
`basic_generate_id` algorithm, which strips all non-ASCII characters and falls
back to `"section"` for purely non-ASCII headings.

Example (Arabic heading inside `<div markdown="1" dir="rtl">`):
- **Jekyll (correct)**: `<h2 id="section">` (basic_generate_id strips Arabic)
- **rustkyll (current)**: `<h2 id="ما-معنى-أن-تكون-مسؤول-عن-مشروع">`
  (GFM algorithm preserves Unicode)

Verification that this is kramdown's behavior:
```ruby
# Standalone GFM heading (preserves Unicode):
Kramdown::Document.new('## Arabic heading', input: 'GFM').to_html
# => <h2 id="arabic-heading">...</h2>

# Same heading inside markdown="1" div (falls back to basic_generate_id):
Kramdown::Document.new('<div markdown="1">## Arabic heading</div>', input: 'GFM').to_html
# => <h2 id="section">...</h2>
```

This affects all 27 language translations in opensource-guide (13 pages each =
351 pages) where headings in non-Latin scripts produce `#section`, `#section-1`,
etc. in Jekyll but full Unicode IDs in rustkyll.

### Other diffs (out of scope)

The remaining diffs on these pages include:
- Missing `<li>` in nav (template/include issue, not kramdown)
- Extra `>` text leak in nav (template issue)
- Meta tag ordering (head element positioning)

These are separate issues and not addressed here.

## Scope

### In scope

1. **Fix `markdown="1"` attribute stripping on nested elements** -- when
   processing HTML blocks with `markdown="1"`, recursively handle inner elements
   that also have the attribute. Specifically, `<p markdown="1">` content should
   be processed as inline markdown (span-level parsing) and the attribute
   stripped.

2. **Use `basic_generate_id` for headings inside `markdown="1"` blocks** --
   implement kramdown's basic heading ID algorithm (ASCII-only: strip non-ASCII,
   downcase, replace spaces/special chars with hyphens, fallback to "section")
   and use it for headings generated during `markdown="1"` content processing.

3. **Duplicate ID handling** -- basic_generate_id uses the same deduplication as
   GFM: append `-1`, `-2`, etc. for duplicate IDs. Both algorithms share the
   same ID tracker.

### Out of scope

- Other opensource-guide diffs (nav, meta tag ordering)
- `markdown="block"` processing improvements (if already working)
- Changes to GFM heading ID algorithm for normal content

## Dependencies

- None.

## Key Files to Modify

- `src/kramdown_parser/html.rs` -- `process_markdown_span_in_raw_html()` or
  equivalent function that handles `markdown="1"` attribute processing; needs to
  handle nested elements recursively
- `src/kramdown.rs` -- `add_heading_ids()` or the post-processing step; may need
  a separate `basic_generate_id()` function and a way to use it for headings
  produced by `markdown="1"` block processing

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `<p markdown="1">` content is processed as inline markdown (links rendered)
      and the `markdown="1"` attribute is stripped from output
- [ ] `<div markdown="1">` containing `<p markdown="1">` processes both levels
- [ ] Headings inside `markdown="1"` blocks get IDs from `basic_generate_id`
      (non-ASCII stripped, fallback to "section")
- [ ] Arabic heading in `<div markdown="1">` gets `id="section"`, not Unicode ID
- [ ] Second Arabic heading in same block gets `id="section-1"`
- [ ] Latin heading in `<div markdown="1">` still gets a proper slug (e.g.,
      `id="my-heading"`)
- [ ] Normal headings (outside `markdown="1"` blocks) continue to use GFM IDs
      (Unicode preserved)
- [ ] opensource-guide DOM comparison improves from 23/388 to 150+/388
- [ ] No regressions: DTC remains 745+/790, muan-blog remains 2174+/2218
- [ ] All sites currently at 100% remain at 100%
- [ ] Tests include non-ASCII/Unicode content

## Test Scenarios

### Unit: `markdown="1"` on nested `<p>` elements

- Input: `<aside markdown="1"><p markdown="1">text [link](url)</p></aside>`
  Output: `<aside><p>text <a href="url">link</a></p></aside>`
- Input: `<div markdown="1"><p markdown="1">**bold**</p></div>`
  Output contains: `<div><p><strong>bold</strong></p></div>`
- Input: `<aside markdown="1"><p markdown="1" class="credit">-- @user</p></aside>`
  Output: attribute stripped, class preserved: `<p class="credit">-- @user</p>`
- Input: `<p markdown="1">no outer block</p>` (standalone)
  Output: attribute stripped, content processed

### Unit: basic_generate_id algorithm

- ASCII text: `"My Heading"` -> `"my-heading"`
- Arabic text: `"ما معنى أن تكون"` -> `"section"` (all non-ASCII stripped)
- Mixed: `"Code of Conduct مدونة"` -> `"code-of-conduct-"` or similar
  (non-ASCII stripped, ASCII preserved)
- Duplicate handling: two `"section"` headings -> `"section"`, `"section-1"`
- Special chars: `"Hello & World <>"` -> `"hello--world-"` (stripped)

### Unit: heading IDs inside markdown blocks

- Input: `<div markdown="1">\n## Arabic Heading\n</div>` processed with GFM
  parser. Heading inside should get basic_generate_id, not GFM ID.
- Input: Normal `## Arabic Heading` (no markdown="1" wrapper) should still get
  GFM Unicode ID.

### Integration: opensource-guide

- Build opensource-guide with rustkyll
- Verify `best-practices/index.html` has no `markdown="1"` attributes in output
- Verify Arabic pages have `id="section"` style heading IDs
- Run DOM comparison, verify 150+/388 pages match (up from 23)

### Integration: Regression check

- Build DTC, verify no regression
- Build muan-blog, verify no regression
- Run `cargo test` full suite
- Verify all 100% sites remain at 100%

## Output Verification

```bash
./scripts/cargo-safe build --release

# opensource-guide
./target/release/rustkyll build \
  --source websites/opensource-guide/ \
  --destination /tmp/osg_320

# Verify no markdown="1" leaking
grep -c 'markdown="1"' /tmp/osg_320/best-practices/index.html
# Must be 0

# Verify Arabic heading IDs use basic_generate_id
grep 'id="section"' /tmp/osg_320/ar/best-practices/index.html
# Must find matches

# Verify English headings still have proper slugs
grep 'id="' /tmp/osg_320/best-practices/index.html | head -5
# Should show meaningful English slugs

# DOM comparison
uv run scripts/dom_compare.py \
  --jekyll-dir websites/opensource-guide/_site_jekyll_cached \
  --rustkyll-dir /tmp/osg_320
# Target: 150+/388 matched (up from 23)

# Regression
./target/release/rustkyll build \
  --source websites/DataTalksClub/datatalksclub.github.io \
  --destination /tmp/dtc_320
uv run scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_320
# Must remain 745+/790
```
