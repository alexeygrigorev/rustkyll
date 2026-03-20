# Issue 251: Fix jekyll-docs news page liquid leaks

## Problem

`news/index.html` and `news/releases/index.html` in jekyll-docs produce raw Liquid output instead of rendered HTML. Additionally, 3 individual posts render without their layout (fallback to raw HTML). All 5 failures share the same root cause.

## Root Cause

The `array_to_sentence_string` filter in the include templates `news_item.html` and `news_item_archive.html` receives `nil` instead of an array for `post.categories`. This causes a render error, and the generator's fallback mechanism writes raw content (including unprocessed Liquid tags) to the output file.

### Why `post.categories` is nil

Three jekyll-docs posts use `category: release` (singular) in their front matter instead of `categories: [release]` (plural array). In Jekyll, `category: X` is automatically converted to `categories: ["X"]` on every document object, including those in `site.posts` and `site.categories`.

In rustkyll, this conversion happens only in `generate_collection_pages_cached_with_progress` (line ~1135 of `src/generator.rs`) via `normalize_fm_to_array`, which runs when rendering individual collection pages. However, `collection_item_to_liquid_slim` (line ~484 of `src/generator.rs`), which builds the post objects used in `site.posts` and `site.categories.*`, does NOT perform this conversion. So:

1. Post has `category: release` in YAML front matter
2. `collection_item_to_liquid_slim` copies `category` as a scalar string but never creates a `categories` key
3. Template accesses `post.categories` which is nil
4. `nil | array_to_sentence_string` errors with "Array expected"
5. Liquid render error propagates up, causing the entire page render to fail
6. Generator fallback writes raw content (with `{% for %}`, `{% include %}` tags) to output

### Affected pages (5 total)

Individual posts (3) -- rendered through `_layouts/news_item.html` which also uses `page.categories | array_to_sentence_string`:
- `release/2016/10/06/jekyll-3-3-is-here.html` (from `category: release`)
- `release/2018/01/02/jekyll-3-7-0-released.html` (from `category: release`)
- `release/2018/01/25/jekyll-3-7-2-released.html` (from `category: release`)

Standalone pages (2) -- iterate over `site.posts`/`site.categories.release` and include `news_item.html`/`news_item_archive.html`:
- `news/index.html` (iterates all `site.posts`)
- `news/releases/index.html` (iterates `site.categories.release`)

### Error trace from build output

```
Warning: failed to render page 'news', writing fallback: template render error: liquid: Invalid input
  with: cause=Array expected
from: Filter error
  with: filter=array_to_sentence_string, input=nil
from: {% include "news_item_archive.html" %}
```

## Fix

In `collection_item_to_liquid_slim` (and `collection_item_to_liquid_full` if applicable), ensure that:

1. If the front matter has `category` (singular string), convert it to `categories` as a single-element array (matching Jekyll's behavior)
2. If the front matter has `categories` as a string, convert it to an array (matching Jekyll's behavior, already done by `normalize_fm_to_array` for individual page rendering)
3. If neither `category` nor `categories` exists, set `categories` to an empty array (Jekyll always exposes `post.categories` as an array, never nil)

The same logic should apply to `tags`/`tag` for consistency, since `array_to_sentence_string` and other array filters may be used on tags too.

Note: Do NOT simply make `array_to_sentence_string` lenient with nil input. The proper fix is to ensure the data model matches Jekyll's (categories is always an array on every document object). Fixing the data model also fixes the 3 individual post failures and any other template that accesses `post.categories`.

## Acceptance Criteria

- [ ] `collection_item_to_liquid_slim` normalizes `category` (singular) to `categories` (plural array) -- matching Jekyll behavior
- [ ] `collection_item_to_liquid_slim` normalizes string `categories` to a single-element array
- [ ] `collection_item_to_liquid_slim` defaults `categories` to an empty array when neither `category` nor `categories` is present
- [ ] Same normalization applies to `tags`/`tag` for consistency
- [ ] Building jekyll-docs produces NO warnings about `news` or `releases` pages
- [ ] Building jekyll-docs produces NO warnings about `jekyll-3-3-is-here`, `jekyll-3-7-0-released`, or `jekyll-3-7-2-released`
- [ ] `news/index.html` output contains `<head>` and `<body>` elements (not raw Liquid tags)
- [ ] `news/releases/index.html` output contains `<head>` and `<body>` elements (not raw Liquid tags)
- [ ] `news/index.html` output contains rendered `<article>` elements from the include templates
- [ ] `news/releases/index.html` output contains rendered `<article>` elements from the include templates
- [ ] The 3 individual release posts render with their `news_item` layout (contain `<head>`, `<body>`, layout structure)
- [ ] No raw `{%` or `{{` Liquid tags appear in any of the 5 affected output files (outside of `<code>` blocks)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] `cargo clippy -- -D warnings` is clean
- [ ] `cargo fmt` produces no changes

## Test Scenarios

### Unit: category/tag normalization in slim context

- Create a `CollectionItem` with `category: "release"` (singular string), convert via `collection_item_to_liquid_slim`, verify the resulting object has `categories` as an array `["release"]`
- Create a `CollectionItem` with `categories: "food"` (string, not array), convert via `collection_item_to_liquid_slim`, verify `categories` becomes `["food"]`
- Create a `CollectionItem` with `categories: ["a", "b"]` (already an array), verify it remains unchanged
- Create a `CollectionItem` with neither `category` nor `categories`, verify `categories` is an empty array `[]`
- Same 4 tests for `tag`/`tags` normalization

### Unit: array_to_sentence_string with normalized categories

- Render `{{ post.categories | array_to_sentence_string }}` where `post` comes from a `collection_item_to_liquid_slim` with `category: "release"`, verify output is `"release"` (not an error)

### Integration: jekyll-docs news pages render correctly

- Build jekyll-docs site, verify `news/index.html` contains `<article` (from rendered includes)
- Build jekyll-docs site, verify `news/releases/index.html` contains `<article` (from rendered includes)
- Build jekyll-docs site, verify no render warnings for pages `news`, `releases`, `jekyll-3-3-is-here`, `jekyll-3-7-0-released`, `jekyll-3-7-2-released`

## Dependencies

- Issue 230 (done)

## Notes

- The `collection_item_to_liquid_full` function in `src/pagination.rs` may need the same fix if it builds post objects used in paginator contexts
- The `jekyllconf/index.html` page also fails to render but with a different error (`modulo` filter on date string) -- that is a separate issue, not in scope here

## Log

### [SWE] 2026-03-20

TDD cycle:

1. Wrote 8 unit tests in src/generator.rs (test_slim_category_singular_to_categories_array, test_slim_categories_string_to_array, test_slim_categories_array_unchanged, test_slim_no_category_defaults_to_empty_array, test_slim_tag_singular_to_tags_array, test_slim_tags_string_to_array, test_slim_tags_array_unchanged, test_slim_no_tag_defaults_to_empty_array)
2. Ran tests: 6 FAIL as expected -- `categories key must exist` (singular->plural not converted), `categories must be an array` (string not converted to array), `tags key must exist` (no default empty array). 2 PASS (array already unchanged).
3. Implemented `normalize_categories_and_tags()` function in src/generator.rs:
   - Converts `category` (singular string) to `categories` (single-element array)
   - Converts `categories` string to array
   - Defaults to empty array when neither exists
   - Same for `tag`/`tags`
4. Called it from `collection_item_to_liquid_slim` (src/generator.rs) and `collection_item_to_liquid_full` (src/pagination.rs)
5. Ran tests: all 8 new tests PASS
6. Full test suite: 1899 lib tests + all integration tests pass (0 failures)
7. clippy: pre-existing vendor/liquid-core warnings only, no new warnings in rustkyll code
8. fmt: clean after auto-format

Files modified:
- src/generator.rs -- added `normalize_categories_and_tags()` function, `ValueView` import, call in `collection_item_to_liquid_slim`, 8 new unit tests
- src/pagination.rs -- call `normalize_categories_and_tags` in `collection_item_to_liquid_full`
- docs/tracker/251-fix-jekyll-docs-news-liquid-leaks.in-progress.md -- status + log

### [SWE] 2026-03-20 QA fix

QA found 3 individual release posts still failing: the page rendering path in
`generate_collection_pages_cached_with_progress` called `normalize_fm_to_array`
for `categories`/`tags` but did NOT convert singular `category`/`tag` to plural
first. The `normalize_categories_and_tags` function (for Liquid Objects) already
handled this, but the FrontMatter (serde_yaml) path did not.

TDD cycle:
1. Wrote 2 new tests: `test_page_fm_singular_category_to_categories_array`,
   `test_page_fm_singular_tag_to_tags_array` -- testing the FrontMatter normalization path
2. Added singular->plural conversion before `normalize_fm_to_array` calls in
   `generate_collection_pages_cached_with_progress` (line ~1198 of src/generator.rs)
3. All tests pass (1901 lib tests + all integration tests, 0 failures)
4. clippy: pre-existing vendor warnings only
5. fmt: clean

Verification with `recount-all-dom.sh --site jekyll-docs/docs`:
- DOM: 14/125 matches (up from previous)
- Liquid leaks: 44 (unchanged -- the 5 fixed pages were not counted as leaks in recount)
- No warnings about news, releases, or individual release posts
- `news/index.html`: has `<head>`, 6 `<article>` elements, 0 liquid leaks
- `news/releases/index.html`: has `<head>`, 3 `<article>` elements, only `{%` is inside `{% raw %}` blocks (correct content about Jekyll tags)
- All 3 release posts (`jekyll-3-3-is-here.html`, `jekyll-3-7-0-released.html`, `jekyll-3-7-2-released.html`): proper `<!DOCTYPE html>` with `<head>`, 0 liquid leaks
