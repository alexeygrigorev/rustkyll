# Issue 52: Fix jekyll-docs site build

## Problem

jekyll-docs/docs (228 pages) builds with Jekyll in 3.0s but fails with rustkyll. The build fails because the `{% avatar %}` tag (from the `jekyll-avatar` plugin) is not implemented. A secondary issue is that `date_to_long_string` is handled as a passthrough filter rather than being properly implemented, which means dates will render incorrectly in the tutorials layout.

## Root Cause

The `{% avatar %}` tag is used in 4 templates:
- `_includes/news_item.html` -- `{% avatar user=author size=24 -%}`
- `_includes/news_item_archive.html` -- `{% avatar user=author size=24 -%}`
- `_layouts/tutorials.html` -- `{% avatar user=author size=24 %}`
- `_layouts/news_item.html` -- `{% avatar user=author size=24 -%}`

The `jekyll-avatar` plugin generates an `<img>` tag that loads a GitHub user's avatar from `https://avatars.githubusercontent.com/{username}?v=4&s={size}`.

## Scope

1. Implement the `{% avatar %}` custom tag
2. Implement the `date_to_long_string` filter properly (not as passthrough)
3. Register both in the template engine
4. Verify jekyll-docs builds end-to-end

## Implementation Details

### Avatar Tag

The `{% avatar %}` tag supports these forms:
- `{% avatar USERNAME %}` -- avatar for literal username, default size
- `{% avatar user=variable %}` -- avatar from a variable
- `{% avatar user=variable size=N %}` -- avatar with explicit pixel size

Output format (from the real jekyll-avatar plugin):
```html
<img class="avatar avatar-small" src="https://avatars.githubusercontent.com/USERNAME?v=4&amp;s=40" alt="USERNAME" srcset="https://avatars.githubusercontent.com/USERNAME?v=4&amp;s=40 1x, https://avatars.githubusercontent.com/USERNAME?v=4&amp;s=80 2x, https://avatars.githubusercontent.com/USERNAME?v=4&amp;s=120 3x, https://avatars.githubusercontent.com/USERNAME?v=4&amp;s=160 4x" width="40" height="40" />
```

The default size is 40. The class varies: `avatar-small` for sizes <= 48, no size class otherwise.

### date_to_long_string Filter

Format: `DD Month YYYY` (e.g., "27 March 2013"). With `"ordinal"` argument: `27th March 2013`.

## Dependencies

- Issue 48 (retest-all-sites) -- done

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (all existing tests, plus new tests for this issue)
- [ ] The `{% avatar user=variable size=N %}` tag is implemented and registered in the template engine
- [ ] The `{% avatar USERNAME %}` form (literal username) is supported
- [ ] The `date_to_long_string` filter is properly implemented (not passthrough), producing "DD Month YYYY" format
- [ ] Running `cargo run --release -- build --source websites/jekyll-docs/docs` completes without errors
- [ ] The output page count is close to 228 pages (within 5%)
- [ ] No regressions on currently-passing sites (datatalksclub, just-the-docs, al-folio)
- [ ] All existing tests still pass

## Test Scenarios

### Unit: Avatar tag parsing
- Parse `{% avatar parkr %}` -- produces img tag with src pointing to `https://avatars.githubusercontent.com/parkr?v=4&s=40`
- Parse `{% avatar user=author size=24 %}` -- when `author` resolves to "jekyllbot", produces img with `s=24` and correct username
- Parse `{% avatar user=author %}` with no size -- defaults to size 40
- Output includes `class="avatar avatar-small"` for size <= 48
- Output includes `srcset` with 1x, 2x, 3x, 4x variants
- Output includes `width` and `height` attributes matching the requested size
- Whitespace trimming with `-%}` works correctly

### Unit: date_to_long_string filter
- `"2013-03-27"` produces `"27 March 2013"`
- `"2001-09-11"` produces `"11 September 2001"`
- Ordinal mode: `"2013-03-27" | date_to_long_string: "ordinal"` produces `"27th March 2013"`
- Day "01" produces `"1st"` in ordinal mode, "02" produces `"2nd"`, "03" produces `"3rd"`, etc.

### Integration: jekyll-docs site build
- Build `websites/jekyll-docs/docs` with rustkyll and verify it completes without error
- Verify generated HTML files exist for news items and tutorials (the pages that use avatar)
- Verify avatar `<img>` tags appear in the rendered HTML output
- Verify `date_to_long_string` output appears correctly in tutorials layout

## Output Verification

After fixing the build, structurally compare rustkyll output against Jekyll output:

1. Same HTML files generated (file tree diff)
2. For each HTML file, compare structural elements: title, headings (h1-h6), links, images
3. No missing pages, no empty pages, no raw Liquid tags in output
4. RSS/Atom feeds and sitemap (if any) must match
