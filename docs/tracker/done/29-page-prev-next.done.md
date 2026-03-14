# Issue 29: page.previous and page.next

## Problem

Jekyll provides `page.previous` and `page.next` for posts, allowing prev/next navigation. Not implemented in rustkyll.

## Requirements

- Sort posts by date (ascending, matching Jekyll's order)
- Inject `page.previous` and `page.next` into each post's template context
- Each should be a full post object (with `url`, `title`, `date`, `slug`, and all front matter fields)
- First post (oldest) has no `previous`, last post (newest) has no `next`
- `page.previous` / `page.next` must be nil (not an empty object) when absent, so `{% if page.previous %}` works correctly in templates
- All existing tests must continue to pass

## Implementation Notes

- In Jekyll, posts are sorted chronologically (oldest first). `page.previous` points to the older post and `page.next` to the newer post.
- The injection must happen in `generate_collection_pages()` in `src/generator.rs`. Currently the function iterates items with `par_iter()` and builds `page_fm` per-item. The prev/next references need to be computed before parallel iteration (since they depend on the sorted order of all posts).
- Only the `"posts"` collection should get `previous`/`next`. Other collections (people, books, podcast, events) should not.
- Each `previous`/`next` value injected into the page context must be a YAML mapping (or Liquid object) containing at minimum: `url`, `title`, `date`, and any other front matter keys the post has. This allows templates to write `{{ page.previous.title }}` or `{{ page.next.url }}`.
- The podcast layout in the reference site (`datatalksclub.github.io/_layouts/podcast.html`) implements prev/next manually via Liquid loops within a season. This issue is about the built-in Jekyll `page.previous`/`page.next` for posts only, not custom collection navigation.

## Dependencies

- Issue 10 (blog posts) -- must be `.done.md` (already done)
- Issue 05 (collection loader) -- must be `.done.md` (already done)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes with all new and existing tests
- [ ] When generating posts, each post's template context contains `page.previous` and `page.next`
- [ ] `page.previous` on the first (oldest) post is nil
- [ ] `page.next` on the last (newest) post is nil
- [ ] `page.previous.url`, `page.previous.title`, and `page.previous.date` are accessible in templates
- [ ] `page.next.url`, `page.next.title`, and `page.next.date` are accessible in templates
- [ ] Posts are sorted by date ascending (oldest first) before assigning prev/next, matching Jekyll's behavior
- [ ] Non-post collections (people, books, podcast, events) are not affected -- they do not get `previous`/`next` injected
- [ ] A template using `{% if page.next %}<a href="{{ page.next.url }}">{{ page.next.title }}</a>{% endif %}` renders correctly
- [ ] The implementation is generic -- no site-specific hardcoding

## Test Scenarios

### Unit: prev/next assignment logic

- Given 3 posts sorted by date [A=2024-01-01, B=2024-02-01, C=2024-03-01]:
  - Post A has `previous=nil`, `next=B`
  - Post B has `previous=A`, `next=C`
  - Post C has `previous=B`, `next=nil`
- Given 1 post: `previous=nil` and `next=nil`
- Given 2 posts [A=2024-01-01, B=2024-02-01]:
  - Post A has `previous=nil`, `next=B`
  - Post B has `previous=A`, `next=nil`
- Verify that `previous.url` and `previous.title` contain the correct values from the referenced post
- Verify that `next.url` and `next.title` contain the correct values from the referenced post

### Unit: sorting correctness

- Posts with out-of-order dates are sorted correctly before prev/next assignment
- Posts with the same date get deterministic prev/next assignment (e.g., secondary sort by slug or filename)

### Unit: collection type filtering

- Calling `generate_collection_pages` for a non-post collection (e.g., "people") does NOT inject `previous` or `next` into the page front matter

### Integration: template rendering with prev/next

- Render a post template containing `{{ page.next.title }}` -- verify it outputs the next post's title
- Render a post template containing `{% if page.previous %}yes{% else %}no{% endif %}` for the first post -- verify it outputs "no"
- Render a post template containing `{% if page.next %}yes{% else %}no{% endif %}` for the last post -- verify it outputs "no"
- Render a post template containing `{{ page.previous.url }}` for a middle post -- verify it outputs the correct URL

### Edge cases

- Posts with no date in front matter but date parsed from filename should still get correct prev/next
- Empty posts collection: no errors, no prev/next injected
