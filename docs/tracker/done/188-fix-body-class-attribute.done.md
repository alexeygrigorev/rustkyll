# Issue 188: Fix body class attribute computation

## Checklist Category

**Body class attribute differs** -- 764 pages (muan-blog).

## Problem

The `class` attribute on `<body>` elements differs. Jekyll outputs `class='col-pages post-body'` but rustkyll outputs `class='col- post-body'` (missing the collection name in the class).

Sample diff (muan-blog):
```
body: attribute_differs
  expected: "class='col-pages post-body'"
  actual:   "class='col- post-body'"
```

The `col-` prefix should include the collection name (e.g., `col-pages`, `col-posts`), but rustkyll produces an empty collection name resulting in `col-`.

## Goal

Correctly compute page-type CSS classes that depend on the page's collection membership, so the `<body>` class attribute matches Jekyll.

## Affected Sites

- muan-blog: 764 pages affected

## Dependencies

None.

## Approach (TDD)

1. Write a test that renders a page from the "pages" collection and asserts the body class contains `col-pages`
2. Verify the test fails (currently produces `col-`)
3. Fix the template context to correctly expose the page's collection label for body class computation
4. Verify the test passes

## Acceptance Criteria

- [ ] Pages in the `pages` collection get `class='col-pages ...'` on `<body>`
- [ ] Pages in the `posts` collection get `class='col-posts ...'` on `<body>`
- [ ] Custom collection pages get the correct collection label in the body class
- [ ] The `page.collection` Liquid variable is correctly populated for all collection pages
- [ ] muan-blog DOM match improves by ~764 pages
- [ ] `cargo test` passes

## Test Scenarios

### Unit: Body class with collection name (write FIRST, must fail before fix)

- **Test `test_body_class_pages_collection`**: Create a page in the `pages` collection. Render its template that uses `page.collection` for body class. Assert output contains `col-pages`.
- **Test `test_body_class_posts_collection`**: Create a post. Assert body class contains `col-posts`.
- **Test `test_body_class_custom_collection`**: Create a page in a custom collection named `recipes`. Assert body class contains `col-recipes`.
- **Test `test_page_collection_variable`**: Assert `{{ page.collection }}` outputs the correct collection label (e.g., `pages`, `posts`).

### Regression: Other body class components preserved

- **Test `test_body_class_other_parts_unchanged`**: Verify other parts of the body class (e.g., `post-body`, layout-specific classes) are still generated correctly.

### Integration: Output verification

- Build muan-blog and inspect several pages to verify body class attributes match Jekyll.
- Check pages from different collections (pages, posts) to verify collection-specific classes.

## Log

### [SWE] 2026-03-17

- Root cause: `page.collection` Liquid variable was never injected into the page front matter context before rendering. The `CollectionItem` struct already had a `collection_name` field, but it was not being added to `page_fm` in the rendering pipeline.
- Fix applied in 3 locations in `src/generator.rs`:
  1. `generate_collection_pages_cached_with_progress`: inject `collection` into `page_fm` from `item.collection_name`
  2. `generate_pages_cached_with_config_and_progress`: inject `collection = "pages"` for standalone pages
  3. `collection_item_to_liquid_slim`: include `collection` field in cross-reference Liquid objects
- Tests added: 5 unit tests
  - `test_body_class_pages_collection`: pages collection -> col-pages
  - `test_body_class_posts_collection`: posts collection -> col-posts
  - `test_body_class_custom_collection`: custom "recipes" collection -> col-recipes
  - `test_page_collection_variable`: `{{ page.collection }}` outputs correct label
  - `test_collection_item_to_liquid_slim_includes_collection`: slim representation includes collection
- TDD followed: wrote failing test first, verified `col-` output, then implemented fix
- Build: 1425 passed, 0 failed from my changes (1 pre-existing failure in syntax.rs from another agent's work)
- Clippy clean, fmt clean
- Files modified: `src/generator.rs`
