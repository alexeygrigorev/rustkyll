# Issue 495: Category sort order conflict (first-encounter vs alphabetical)

## Problem

Issue #354 changed `__key_order` for `site.categories` and `site.tags` to
alphabetical order (to fix hydeout sidebar nav). Issue #399 originally used
first-encounter order (to fix large-blog-3000). The current code has
alphabetical order, which regressed large-blog-3000 from 3001/3001 to
3000/3001 (index.html has 54 diffs from wrong category iteration order).

## Root Cause (investigated during grooming)

**Jekyll uses first-encounter order, period.** There is no context-dependent
ordering. The Jekyll source (`lib/jekyll/site.rb`, method `post_attr_hash`)
builds `site.categories` and `site.tags` using a Ruby `Hash`, which preserves
insertion order (Ruby 1.9+). Posts are iterated via `posts.docs.each` in
date-ascending order (sorted by `Document#<=>`, which sorts by date then path).
So categories appear in the order their first post is encountered when scanning
posts oldest-to-newest.

The `site_drop.rb` exposes `categories` and `tags` directly via
`delegate_methods` with no additional sorting. Templates that want alphabetical
order (like hydeout) sort explicitly in Liquid -- hydeout uses
`site.pages|sort:"sidebar_sort_order"` to list category pages, it never
iterates `site.categories` directly for nav.

**Issue #354's alphabetical change was wrong.** The hydeout fix did not
actually need alphabetical `__key_order` on `site.categories` -- the hydeout
sidebar sorts `site.pages`, not `site.categories`. The alphabetical change was
a misdiagnosis that happened to produce the right output for hydeout but
broke the contract for sites that iterate `site.categories` directly (like
large-blog-3000).

## Fix Required

1. Revert the alphabetical sorting of `__key_order` in
   `build_categories_and_tags_from_liquid()` (generator.rs lines ~1120-1153).
   Instead, build `__key_order` from the IndexMap's iteration order, which is
   already first-encounter order.

2. Do the same for the dead-code `build_categories_and_tags()` function
   (~lines 1191-1215) for consistency.

3. Update the existing tests that assert alphabetical order
   (`test_issue399_categories_key_order_metadata`,
   `test_issue399_tags_key_order_metadata`,
   `test_issue399_categories_key_order_with_duplicates`) to assert
   first-encounter order instead.

4. Verify hydeout is NOT regressed (it should not be, since hydeout sorts
   `site.pages` not `site.categories`).

## Dependencies

None. This is a standalone fix.

## Baseline

- DTC: 790/790
- large-blog-3000: 3000/3001 (target: restore to 3001/3001)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` reports no changes
- [ ] `__key_order` for `site.categories` uses first-encounter order (order each category is first seen when processing posts date-ascending), NOT alphabetical
- [ ] `__key_order` for `site.tags` uses first-encounter order, NOT alphabetical
- [ ] large-blog-3000 DOM comparison reaches 3001/3001 (restored from 3000/3001)
- [ ] DTC DOM match count does not drop below 790/790
- [ ] Hydeout DOM match count does not drop (confirm it was not relying on alphabetical `site.categories`)
- [ ] Existing tests pass (updated to expect first-encounter order)
- [ ] Posts within each category remain in reverse chronological order (unchanged)

## Test Scenarios

### Unit: Category key order is first-encounter

- Create posts with categories in non-alphabetical encounter order (e.g., Technology at 2024-01-01, Science at 2024-01-02, Travel at 2024-01-03). Verify `__key_order` is `["Technology", "Science", "Travel"]`, NOT `["Science", "Technology", "Travel"]`.
- Same test for tags: tags encountered as rust, python, ml should have `__key_order` `["rust", "python", "ml"]`, NOT `["ml", "python", "rust"]`.

### Unit: Duplicates appear once at first encounter position

- Posts: p1 has categories [A, B], p2 has [B, C], p3 has [A, C, D]. First-encounter order is A, B, C, D. Verify `__key_order` is `["A", "B", "C", "D"]` (this happens to be alphabetical for this test case, but the ordering logic should be encounter-based, not sorted).

### Unit: Reverse chronological within category (unchanged)

- Multiple posts in the same category. Verify posts within the category array are newest-first. This is existing behavior that must not regress.

### Integration: large-blog-3000 DOM comparison

- Build large-blog-3000 with rustkyll and compare against Jekyll cached output.
- index.html must match (the 54 category-order diffs must be resolved).
- DOM match must reach 3001/3001.

### Integration: Hydeout non-regression

- Build hydeout and compare against Jekyll cached output.
- DOM match count must not drop.

### Integration: DTC non-regression

- Build DTC site and compare against Jekyll output.
- DOM match must stay at 790/790.

## Implementation Hints

The fix is small. In `build_categories_and_tags_from_liquid()` around line 1120
of `src/generator.rs`, replace:

```rust
let mut cat_keys_sorted: Vec<&String> = categories.keys().collect();
cat_keys_sorted.sort();
let cat_key_order: Vec<LiquidValue> = cat_keys_sorted
    .iter()
    .map(|k| LiquidValue::scalar((*k).clone()))
    .collect();
```

with:

```rust
let cat_key_order: Vec<LiquidValue> = categories
    .keys()
    .map(|k| LiquidValue::scalar(k.clone()))
    .collect();
```

Same pattern for tags. The IndexMap already preserves first-encounter order,
so just iterate its keys directly without sorting.

## Log

### [SWE] 2026-03-29
- TDD step 1: Updated 3 test assertions to expect first-encounter order instead of alphabetical
- TDD step 2: Ran tests -- 2 FAILED as expected (categories_key_order_metadata, tags_key_order_metadata)
  - Got ["Science", "Technology", "Travel"], expected ["Technology", "Science", "Travel"]
  - Got ["ml", "python", "rust"], expected ["rust", "python", "ml"]
- TDD step 3: Removed `.sort()` calls for cat_keys_sorted and tag_keys_sorted in `build_categories_and_tags_from_liquid()`, replaced with direct IndexMap key iteration
- TDD step 4: All 4 issue399 tests PASS
- Full test suite: all tests pass
- Clippy: clean (no warnings)
- Fmt: clean (no changes)
- DOM verification:
  - DTC: 790/790 (no regression)
  - large-blog-3000: 3001/3001 (restored from 3000/3001)
  - hydeout: 17/30 (no regression, same as before)
- Files modified: src/generator.rs (only file changed)
- Changes: removed alphabetical sort of __key_order, updated comments, updated 3 test assertions
