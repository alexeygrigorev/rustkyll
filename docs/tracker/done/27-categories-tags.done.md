# Issue 27: site.categories and site.tags

## Problem

Posts have categories and tags in front matter, but rustkyll does not build `site.categories` or `site.tags` mappings. Templates that iterate over `site.categories` or `site.tags` (e.g., tag cloud pages, category index pages) will fail or produce empty output.

## Requirements

- Build `site.categories` as a hash mapping category name to an array of posts in that category
- Build `site.tags` as a hash mapping tag name to an array of posts with that tag
- Extract categories from both front matter `categories`/`category` fields (already done in `extract_categories()`)
- Extract tags from front matter `tags`/`tag` fields (analogous to categories)
- Expose both in the template context as `site.categories` and `site.tags`
- Each post in the arrays must be a full Liquid object (same shape as items in `site.posts`)
- All existing tests must continue to pass

## Scope

- `src/collection.rs` -- add `extract_tags()` function (parallel to existing `extract_categories()`)
- `src/generator.rs` -- in `build_site_context()`, build and insert `site.categories` and `site.tags` hashes from the posts collection
- Tests in both modules

## Dependencies

- No strict dependencies. The `extract_categories()` function already exists in `collection.rs`. The `build_site_context()` function in `generator.rs` already has access to all collections.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] An `extract_tags()` function exists that extracts tags from front matter, supporting both `tags: [a, b]` (array) and `tag: x` (single string) formats
- [ ] `build_site_context()` populates `site.categories` as a Liquid object where each key is a category name and the value is an array of post objects
- [ ] `build_site_context()` populates `site.tags` as a Liquid object where each key is a tag name and the value is an array of post objects
- [ ] Posts with multiple categories appear in each category's array
- [ ] Posts with multiple tags appear in each tag's array
- [ ] Posts with no categories or tags do not cause errors and are simply absent from the mappings
- [ ] The post objects inside `site.categories["some-cat"]` have the same structure as posts in `site.posts` (url, title, date, content, slug, etc.)
- [ ] Only posts (not other collections like people or books) are included in `site.categories` and `site.tags` -- this matches Jekyll behavior
- [ ] When iterating `{% for cat in site.categories %}`, `cat[0]` is the category name and `cat[1]` is the array of posts (Jekyll hash iteration convention)
- [ ] The mappings work with the DTC site's actual posts (many have tags, none have categories in front matter currently)

## Test Scenarios

### Unit: extract_tags

- Parse front matter with `tags: ["machine-learning", "tutorial"]` -- verify both tags returned
- Parse front matter with `tag: "python"` (single string fallback) -- verify `["python"]` returned
- Parse front matter with `tags: "single-tag"` (string instead of array) -- verify `["single-tag"]` returned
- Parse front matter with no tags or tag key -- verify empty vec returned
- Parse front matter with both `tags` and `tag` -- verify `tags` takes precedence (array form wins)
- Parse front matter with empty `tags: []` -- verify empty vec returned

### Unit: extract_categories (existing, verify no regression)

- Existing tests for `extract_categories` continue to pass (array form, single form, empty)

### Unit: build_site_context categories mapping

- Create 3 posts: post A with categories `["ml", "python"]`, post B with category `"ml"`, post C with no categories
- Build site context and verify `site.categories` is an object with keys `"ml"` and `"python"`
- Verify `site.categories["ml"]` contains posts A and B (2 posts)
- Verify `site.categories["python"]` contains only post A (1 post)
- Verify post C does not appear in any category

### Unit: build_site_context tags mapping

- Create 3 posts: post A with tags `["data-science", "career"]`, post B with tags `["data-science"]`, post C with no tags
- Build site context and verify `site.tags` is an object with keys `"data-science"` and `"career"`
- Verify `site.tags["data-science"]` contains posts A and B
- Verify `site.tags["career"]` contains only post A
- Verify post C does not appear in any tag

### Unit: empty posts collection

- Build site context with no posts collection at all -- verify `site.categories` and `site.tags` are empty objects (not missing)

### Unit: non-post collections excluded

- Build site context with a "people" collection that has items with `tags` in front matter -- verify those items do NOT appear in `site.tags`

### Integration: DTC site categories and tags

- Load real DTC posts, build site context, verify `site.tags` contains known tags from the actual post front matter (e.g., pick a tag from `_posts/2020-11-29-segmentation.md` and verify that post appears in that tag's array)
- Verify `site.categories` is an empty object (since DTC posts do not use categories in front matter)

## References

- Issue #22 compatibility research, gap #8
- Jekyll docs: https://jekyllrb.com/docs/posts/#categories-and-tags
