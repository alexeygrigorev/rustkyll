# Issue 461: Pre-compute where filter indexes for collection lookups

## Problem

DTC podcast pages call the `where` Liquid filter 15+ times per page
against 428-item arrays (site.people, site.podcasts). Each call is O(n)
linear scan. With 194 podcast pages, this is ~2,900 linear scans.

## Approach

Build hash indexes during `CachedSiteContext` construction:
- For each collection array, create `HashMap<(field_name, value), Vec<&Item>>`
- When `where` filter is called, check if an index exists and use O(1) lookup
- Fall back to linear scan for non-indexed collections

## Expected Impact

Podcast pages are ~55% of DTC generation time. The `where` filter is
the dominant cost per page. Pre-computing indexes could reduce this
by 20-30%, bringing DTC from ~1.0s toward ~0.7s.

## Acceptance Criteria

- [ ] DTC build time < 0.75s (median of 3 runs)
- [ ] DTC DOM stays at 790/790
- [ ] No regression on any other site
- [ ] Only accept if measurable >20% improvement

## Files

- `src/template/filters/where_filter.rs` -- the where filter implementation
- `vendor/liquid-core/src/runtime/stack.rs` -- StackFrame::get() optimization

## Log

### [SWE] 2026-03-28

**TDD cycle:**

1. Wrote 7 new tests for the indexed where filter path (large array single match, multiple matches, no match, missing field, unicode values, direct index build test)
2. Ran tests: all PASS (filter correctness verified before optimization)
3. Baseline benchmark: DTC median 1.06s (3 runs: 1.060, 1.031, 1.126)

**Implementation (two-part optimization):**

Part 1: Thread-local index cache in `where_filter.rs`
- Added `HashMap<usize, HashMap<String, HashMap<String, Arc<Vec<Value>>>>>` thread-local cache
- Cache key: data pointer of input `&dyn ValueView` + field name
- On first call for (array, field), builds full field index; subsequent calls use O(1) lookup
- Arrays below 16 items fall back to linear scan (no cache overhead)
- Initial result: 0.92s (13% improvement) -- cache was not hitting because Liquid runtime was cloning arrays via `into_owned()`

Part 2: Vendor fix in `liquid-core/src/runtime/stack.rs`
- Removed unnecessary `into_owned()` in `StackFrame::get()`
- `StackFrame` (unlike `GlobalFrame`/`IndexFrame` which use `RefCell`) owns `data` directly via `&self`, so borrowed `ValueCow` has valid lifetime
- This makes collection array references pointer-stable across renders, enabling the cache
- After fix: cache shows ~24 misses (one per thread per field) vs ~2,500 hits

**Results:**
- DTC build: median 0.694s (3 runs: 0.681, 0.694, 0.770) -- 35% improvement from 1.06s
- DTC DOM: 790/790 (unchanged)
- All tests pass: 3066 lib + integration tests, 0 failures
- Clippy clean, fmt clean

**Files modified:**
- `src/template/filters/where_filter.rs` -- thread-local index cache + 7 new tests
- `vendor/liquid-core/src/runtime/stack.rs` -- remove `into_owned()` in StackFrame::get()
