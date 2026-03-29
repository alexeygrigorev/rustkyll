# Issue 354: Hydeout category URL casing and pagination path

## Problem

Two URL generation issues found with the Hydeout theme:

1. **Category URL casing**: Jekyll generates lowercase category paths (e.g., `edge case/`, `markup/`) while rustkyll preserves the original case from front matter (e.g., `Edge Case/`, `Markup/`). This causes category-based post URLs to differ.

2. **Pagination path**: Jekyll generates pagination pages at root (`/page2/`, `/page3/`) per the default `paginate_path: '/page:num'` setting, while rustkyll generates them at `/blog/page2/` etc.

3. **Future date posts**: Rustkyll includes posts with future dates (e.g., year 9999), while Jekyll skips them by default. This affects the homepage post listing.

Related to issue #241 (Hydeout theme support).

## Impact

- 21 files only in Jekyll / 22 files only in rustkyll (due to case differences and pagination path)
- Homepage post listing shows wrong first post (future-dated post instead of most recent real post)

## Log

### [SWE] 2026-03-29

#### Fix 1: Category URL lowercasing
- Wrote tests: test_category_url_lowercased_edge_case, test_category_url_lowercased_markup, test_multiple_categories_all_lowercased, test_already_lowercase_categories_unchanged
- Tests failed as expected (categories not lowercased in URLs)
- Implemented fix in src/collection.rs:293 -- lowercase categories in generate_url_with_context
- Tests pass: categories correctly lowercased in URLs

#### Fix 2: Pagination path default
- Verified Jekyll's actual default is /blog/page:num/ (matching rustkyll's current default)
- The Hydeout cached output at page2/ was from when paginate_path was uncommented in config
- No change needed -- current default is correct per Jekyll

#### Fix 3: Future-dated post filtering
- Wrote tests: test_filter_future_posts_excludes_future, test_filter_future_posts_includes_when_future_true, test_filter_future_posts_keeps_past_posts
- Tests failed as expected (filter_future_posts function didn't exist)
- Implemented filter_future_posts in src/collection.rs, called from src/main.rs after loading collections
- Tests pass: future posts excluded by default, included when future: true

#### Fix 4: Category nav sort order
- Investigated: Hydeout template uses site.pages|sort:"sidebar_sort_order", not site.categories
- Fixed __key_order for site.categories and site.tags to use alphabetical order (src/generator.rs)
- Fixed sort filter (src/template/filters/sort.rs) to preserve original order when sort property is missing from all items (was falling through to value-based sort)
- Updated existing tests to expect alphabetical order

#### Results
- Build: 3107 lib tests pass, 0 fail, clippy clean, fmt clean
- DTC DOM: 790/790 (no regression)
- DTC docs: 57/57 (no regression)
- Hydeout DOM: 17/30 (up from 0/13), File match: 34/34 (up from 13 common files)
- Hydeout homepage first post: "Hello Hydeout" (correct, not future-dated post)
- Category nav order: "Edge Case" before "Markup" (alphabetical, matching Jekyll)

#### Files modified
- src/collection.rs (category URL lowercasing, filter_future_posts function, tests)
- src/main.rs (call filter_future_posts after loading collections)
- src/generator.rs (alphabetical __key_order for categories/tags, fix pre-existing compilation errors)
- src/template/filters/sort.rs (preserve order when sort property missing)
