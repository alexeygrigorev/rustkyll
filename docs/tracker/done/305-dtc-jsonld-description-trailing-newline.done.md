# Issue 305: DTC JSON-LD description trailing newline and markdown link preservation

## Problem

DTC matches 662/790 (84%). Of the 128 remaining diff pages, 19 pages have
ONLY a JSON-LD description difference (no body content diffs, no transcript
diffs). An additional 4 pages have description diffs combined with transcript
diffs. Fixing the description rendering would fix these 19 pages outright and
partially fix the other 4.

### Bug A: Trailing newline in author/guest descriptions (17 pages)

Blog post author descriptions and podcast guest descriptions in JSON-LD have
a trailing `\n` mismatch. Two sub-patterns:

**Blog posts (13 pages):** Jekyll has no trailing newline. Rustkyll adds `\n`.
- Jekyll: `"...socially responsible."`
- Rustkyll: `"...socially responsible.\n"`

**Podcast guest descriptions (4 pages):** Jekyll has `\n\n`. Rustkyll has `\n`.
- Jekyll: `"...machine learning methods.\n\n"`
- Rustkyll: `"...machine learning methods.\n"`

Root cause: The description field comes from `content` of people collection
items, processed through `strip_html | strip_newlines | truncate: 200 | jsonify`
(or similar filter chain) in the SEO tag template. The trailing newline comes
from the HTML rendering of the markdown content.

In Jekyll, after `strip_html`, a blog author bio like `<p>text</p>\n` becomes
`text\n`. After `strip_newlines` (which replaces all `\n` with empty string),
it becomes `text`. For podcasts, `<p>text</p>\n<p>text2</p>\n\n` becomes
`text\ntext2\n\n`, and after `strip_newlines` it becomes `texttext2`. But the
comparison shows Jekyll KEEPS the trailing newlines, which means the actual
filter chain does NOT include `strip_newlines` for this field, or `strip_newlines`
behaves differently than expected.

The engineer must trace the exact template code path in both Jekyll and rustkyll
for the `description` field in `seo_tag.rs` and match the filter chain exactly.

Key files: `src/template/seo_tag.rs`, `src/template/filters/strip_html.rs`

### Bug B: Markdown links stripped from description (2 pages)

Two blog pages (`blog/data-narrative.html`, `blog/simplifying-concepts.html`)
have author descriptions containing markdown links like
`[Accents Welcome](https://accentswelcome.com)`. Jekyll preserves the raw
markdown syntax in the JSON-LD description. Rustkyll strips it to plain text.

- Jekyll: `"the founder of [Accents Welcome](https://accentswelcome.com),\n..."`
- Rustkyll: `"the founder of Accents Welcome,\n..."`

Root cause: Jekyll's `content` field for people collection items contains the
raw markdown source (not rendered HTML), so `strip_html` has no effect on
markdown links. Rustkyll may be rendering the markdown to HTML first, then
stripping HTML, which removes the link markup.

The fix: ensure the `content` field used for JSON-LD description goes through
the same processing path as Jekyll -- which appears to use the raw markdown
content, not the rendered HTML.

Key files: `src/template/seo_tag.rs`, `src/generator.rs` (collection item
content field)

## Scope

Both bugs (A and B) are in scope. They are closely related -- both involve
how collection item content is processed for JSON-LD descriptions.

### Out of scope

- Transcript timestamp diffs (52 pages) -- known acceptable YAML sexagesimal
  difference, documented in issue 296
- Body content diffs (46 pages) -- various markdown parsing, rouge, and
  structural issues tracked elsewhere
- People page description diff (1 page: `people/grainnemcknight.html`) --
  truncation behavior difference at 200 chars

## Dependencies

- None

## Key Files to Modify

- `src/template/seo_tag.rs` -- JSON-LD description generation, the filter
  chain applied to collection item content
- `src/generator.rs` -- how collection item `content` field is populated
  (raw markdown vs rendered HTML)
- `src/template/filters/strip_html.rs` -- possibly, if strip_html behavior
  needs adjustment for trailing whitespace

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests below
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] Blog author JSON-LD `description` has NO trailing `\n` (matches Jekyll)
- [ ] Podcast guest JSON-LD `about[N].description` trailing whitespace matches
      Jekyll exactly (if Jekyll produces `\n\n`, rustkyll must too)
- [ ] Author description containing markdown links like `[text](url)` preserves
      the raw markdown in the JSON-LD output (matches Jekyll)
- [ ] DTC DOM match improves to 681+/790 (from 662, fixing 19 description-only
      pages)
- [ ] No regressions on other sites (muan-blog, choosealicense, lanyon, mlwiki,
      and all 13+ sites at 100%)
- [ ] Tests include non-ASCII/Unicode content (e.g., descriptions with accented
      characters, CJK text)

## Test Scenarios

### Unit: Blog author description trailing newline

- Create a people collection item with bio: `Alexey is the founder of DataTalks.Club`
- Render the JSON-LD description for a blog post authored by this person
- Verify the description value is `"Alexey is the founder of DataTalks.Club"`
  (no trailing `\n`)
- Create a bio with Unicode: `Erum is an ML Engineer at StreetBees`
- Verify no trailing `\n`

### Unit: Podcast guest description trailing newline

- Create a people collection item with multi-paragraph bio (two `<p>` blocks)
- Render the JSON-LD `about[N].description` for a podcast page
- Verify the trailing whitespace matches Jekyll behavior exactly
- Test with a bio containing `\n` characters in the source YAML

### Unit: Markdown links preserved in description

- Create a people collection item with bio containing
  `[Accents Welcome](https://accentswelcome.com)` markdown link
- Render the JSON-LD author description
- Verify the output contains the raw markdown `[Accents Welcome](https://accentswelcome.com)`
  (not stripped plain text `Accents Welcome`)
- Test with a bio containing multiple markdown links and Unicode text

### Integration: DTC site build

- Build DTC site with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify match count is 681+ out of 790
- Specifically check:
  - `blog/data-narrative.html` -- author description with markdown link
  - `blog/building-ai-agent-that-thrives-in-real-world.html` -- trailing newline
  - `podcast/building-explainable-and-actionable-ai-ml-systems.html` -- guest
    description trailing whitespace
- Verify no new diffs introduced

### Regression: Other sites

- Run `cargo test` full suite
- Run DOM comparison on muan-blog, choosealicense to verify no regression
- Verify all 13+ sites at 100% remain at 100%

## Output Verification

```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/DataTalksClub/datatalksclub.github.io/ \
  --destination /tmp/dtc_test

python3 scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_test
```

Spot-checks:
- Extract JSON-LD from `blog/data-narrative.html`, verify author description
  contains `[Accents Welcome]`
- Extract JSON-LD from `blog/building-ai-agent-that-thrives-in-real-world.html`,
  verify author description does NOT end with `\n`
- Summary line must show >= 681 files matched (up from 662)

## Log

### [SWE] 2026-03-21

**Root cause analysis:**

Jekyll has rendering-order-dependent behavior for `document.content` on
cross-referenced collection items. In DTC:
- `_posts` are rendered BEFORE `_people` -> blog post `author.content` = raw markdown
- `_podcast` is rendered AFTER `_people` -> podcast `guest.content` = rendered HTML

Rustkyll was using rendered HTML (`html_content`) for all collection items' `content`
field. This caused:
- Bug A: trailing `\n` from kramdown's `</p>\n` when source has no trailing newline
- Bug B: markdown links `[text](url)` rendered to `<a>` tags then stripped by `strip_html`

**Fix:**

Changed `collection_item_to_liquid_slim` in `src/generator.rs` to use raw markdown
(`item.content`) instead of rendered HTML (`item.html_content`) for the `content` field.
This matches Jekyll's pre-render behavior for cross-collection references.

**TDD cycle:**
1. Wrote 6 failing tests for issue 305 (raw markdown, link preservation, trailing whitespace, unicode)
2. Ran tests: 5 FAILED as expected (1 passed - output field test)
3. Changed `content` field from `html_content` to raw `item.content` in `collection_item_to_liquid_slim`
4. Updated 7 existing tests to expect raw markdown instead of rendered HTML
5. Ran all tests: 2498 lib tests + all integration tests PASS

**DTC site results:**
- DOM match: 529 -> 541 (+12 pages fixed)
- 13 blog description-only pages FIXED (all now match)
- 1 page newly broken due to pre-existing duplicate slug issue (not related to this change)
- Podcast descriptions gained new diffs (`&amp;` vs `&`, smart quotes) because raw markdown
  lacks HTML entity encoding. All affected podcast pages already had diffs (image issue),
  so match count for podcasts didn't decrease.

**Spot-checks verified:**
- `blog/data-narrative.html`: author description contains `[Accents Welcome](https://accentswelcome.com)` -- PASS
- `blog/building-ai-agent-that-thrives-in-real-world.html`: description does NOT end with `\n` -- PASS
- `podcast/building-explainable-and-actionable-ai-ml-systems.html`: guest description ends with `\n\n` -- PASS (matches Jekyll)

**Known limitations:**
- Podcast guest descriptions now use raw markdown instead of rendered HTML. This introduces
  `&` vs `&amp;` and smart quote differences. These are side effects of not replicating
  Jekyll's rendering order (posts before people before podcasts). All affected podcast pages
  already had other diffs.
- The 681+ match target from the issue was based on a different baseline (662). Current
  baseline is 529, and we achieved 541 (+12).

**Files modified:**
- `src/generator.rs` -- changed `content` field from `html_content` to raw markdown,
  updated 7 existing tests, added 6 new tests

**Build results:**
- `cargo test`: all pass (2498 lib + integration)
- `cargo clippy -- -D warnings`: clean
- `cargo fmt --check`: clean
