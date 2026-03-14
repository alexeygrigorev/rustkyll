# Issue 42: Support `site.related_posts` and `site.pages`

## Problem

Complex site testing (Issue 35) revealed that some Jekyll sites access `site.related_posts` and `site.pages` in templates. These variables are not currently populated in rustkyll's site context.

- `site.related_posts` -- In Jekyll, defaults to the 10 most recent posts (or LSI-computed related posts if lsi is enabled). We only need the simple "10 most recent posts" behavior.
- `site.pages` -- An array of all standalone Page objects (non-collection, non-post pages). Each page object should expose `title`, `url`, `content`, and any front matter fields, matching Jekyll's page object structure.

## Affected Sites

- Hyde (poole/hyde) -- uses `site.related_posts` and `site.pages`

## Requirements

- Populate `site.related_posts` in the site context with the 10 most recent posts (sorted by date descending)
- Populate `site.pages` with standalone page objects (the `Page` structs loaded by `load_pages`)
- Each entry in `site.related_posts` must have the same structure as entries in `site.posts` (title, url, date, content, excerpt, front matter fields)
- Each entry in `site.pages` must expose at minimum: `title`, `url`, and any front matter fields
- LSI-based related posts are out of scope -- always use the "10 most recent posts" fallback

## Scope

This issue touches:
- `src/generator.rs` -- the `build_site_context` function must accept pages and populate both new fields
- `src/main.rs` -- pass the loaded pages to `build_site_context`
- `src/collection.rs` -- may need a `page_to_liquid` conversion function (similar to `collection_item_to_liquid`)

No changes to the template engine, filters, or layout system.

## Dependencies

None. The page loading infrastructure (Issue 14) and post collection support are already done.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `site.related_posts` is available in templates and contains up to 10 posts
- [ ] `site.related_posts` entries are sorted by date descending (most recent first)
- [ ] If fewer than 10 posts exist, `site.related_posts` contains all posts
- [ ] If no posts exist, `site.related_posts` is an empty array (not nil/missing)
- [ ] Each `site.related_posts` entry has the same fields as a `site.posts` entry (title, url, date, excerpt, content, front matter)
- [ ] `site.pages` is available in templates and contains all standalone pages
- [ ] Each `site.pages` entry has at minimum: `title`, `url`, and front matter fields
- [ ] `site.pages` does not include collection items or posts -- only standalone pages (root-level `.md`/`.html` files)
- [ ] If no standalone pages exist, `site.pages` is an empty array (not nil/missing)
- [ ] A template using `{% for post in site.related_posts %}{{ post.title }}{% endfor %}` renders correctly
- [ ] A template using `{% for page in site.pages %}{{ page.title }}{% endfor %}` renders correctly
- [ ] Existing `site.posts` behavior is unchanged

## Test Scenarios

### Unit: site.related_posts population

- Build site context with 15 posts (with dates). Verify `site.related_posts` contains exactly 10 entries.
- Verify the 10 entries are the 10 most recent posts by date.
- Verify entries are sorted by date descending (most recent first).
- Build site context with 5 posts. Verify `site.related_posts` contains all 5.
- Build site context with 0 posts. Verify `site.related_posts` is an empty array.
- Verify each entry in `site.related_posts` has `title`, `url`, and `date` fields.

### Unit: site.pages population

- Build site context with 3 standalone pages. Verify `site.pages` contains 3 entries.
- Verify each entry has `title` and `url` fields.
- Build site context with 0 pages. Verify `site.pages` is an empty array.
- Verify that collection items (e.g., posts, people) do not appear in `site.pages`.

### Integration: Template rendering with site.related_posts

- Create a template `{% for post in site.related_posts %}<a href="{{ post.url }}">{{ post.title }}</a>{% endfor %}`. Render it with a site context containing posts. Verify the output contains links to the most recent posts.
- Create a template `{{ site.related_posts.size }}`. Render with 15 posts. Verify output is `10`.
- Create a template `{{ site.related_posts.first.title }}`. Render and verify it is the title of the most recent post.

### Integration: Template rendering with site.pages

- Create a template `{% for page in site.pages %}<a href="{{ page.url }}">{{ page.title }}</a>{% endfor %}`. Render with 2 standalone pages. Verify the output contains links to both pages.
- Create a template `{{ site.pages.size }}`. Render with 3 pages. Verify output is `3`.

### Regression

- Verify all existing generator tests still pass
- Verify `site.posts` is unchanged (still contains all posts, not just 10)
- Verify the DTC site still builds successfully with the same number of output pages

## Notes

- `build_site_context` currently takes `(config, collections, data, site_dir)`. It will need an additional parameter for standalone pages (e.g., `pages: &[Page]`). Update all call sites accordingly.
- For `site.related_posts`, extract the `posts` collection from the `collections` HashMap, sort by date, take the first 10, and convert to Liquid values using `collection_item_to_liquid`.
- For `site.pages`, create a conversion function similar to `collection_item_to_liquid` that converts a `Page` struct to a Liquid `Value` object with the appropriate fields.
- The `related_posts` should use the same object structure as `site.posts` entries so that templates can use the same field names interchangeably.

## References

- `src/generator.rs` -- `build_site_context`, `collection_item_to_liquid`
- `src/collection.rs` -- `Page` struct, `load_pages`
- `src/main.rs` -- call sites for `build_site_context`
- Jekyll documentation on `site.related_posts`: https://jekyllrb.com/docs/variables/#site-variables
