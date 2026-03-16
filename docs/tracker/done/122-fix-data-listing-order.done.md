# Issue 122: Fix data listing order on DTC tools and course pages

## Priority

HIGH — tools.html (1.27%) and course-ml-zoomcamp.html (4.11%) pixel diff due to listing order.

## Problem

Events and tools appear in different order than Jekyll. This affects:
- /tools.html — tool listing order differs
- /courses/2021-winter-ml-zoomcamp.html — event listing order differs

## Root cause

Data file iteration order or collection sorting differs from Jekyll. Jekyll likely uses insertion order from YAML data files or specific sort criteria.

## Acceptance criteria

- tools.html achieves 0% pixel diff
- course-ml-zoomcamp.html achieves 0% pixel diff
- Data listing order matches Jekyll exactly
- No regressions on other pages

## Log

### [SWE] 2026-03-16 Investigation and Fix

**Investigation findings:**
- Built DTC site with both Jekyll and rustkyll and compared tools.html and course-ml-zoomcamp.html
- The pixel diffs on these pages are NOT actually from data listing order -- content ordering is identical between Jekyll and rustkyll
- The actual diffs are from whitespace/indentation, `&ndash;` vs UTF-8 `--`, and `<br />` vs `<br/>` differences (tracked in other issues like template-whitespace #103 and kramdown-loose-list #124)
- However, a real latent bug existed: `DataTree` used `HashMap<String, serde_yaml::Value>` which produces non-deterministic iteration order due to HashMap randomization
- `load_directory_files` also used `HashMap` and `fs::read_dir` which has no guaranteed ordering
- `liquid::Object` internally uses `HashMap` too, but we cannot change the liquid library

**Root cause (latent bug fixed):**
1. `DataTree = HashMap<String, serde_yaml::Value>` -- non-deterministic iteration order
2. `load_directory_files` returned `HashMap` -- non-deterministic subdirectory key order
3. `fs::read_dir` entries were not sorted -- filesystem-dependent ordering

**Fix applied:**
1. Changed `DataTree` from `HashMap` to `BTreeMap` for deterministic alphabetical key ordering
2. Changed `load_directory_files` to return `BTreeMap`
3. Added explicit sorting of `fs::read_dir` entries by filename in both `load_data` and `load_directory_files`
4. Updated integration tests (`integration_pagination.rs`, `integration_performance.rs`) to use `BTreeMap` for data parameter

**Tests added:** 3 new tests
- `test_data_tree_keys_sorted_alphabetically` -- verifies top-level data keys are alphabetically sorted
- `test_subdirectory_keys_sorted_alphabetically` -- verifies subdirectory (e.g. faqs/) keys are sorted
- `test_data_order_deterministic_across_loads` -- verifies order is consistent across multiple loads

**Build results:** All 1215 tests pass, 0 fail, clippy clean, fmt clean

**Files modified:**
- `src/data.rs` -- BTreeMap, sorted dir entries, 3 new tests
- `tests/integration_pagination.rs` -- BTreeMap import and usage for data param
- `tests/integration_performance.rs` -- BTreeMap import and usage for data param

**Note on acceptance criteria:** The pixel diffs on tools.html and course-ml-zoomcamp.html are not caused by data ordering. They come from template whitespace and Markdown rendering differences (tracked in issues #103, #124). This fix addresses the underlying data ordering correctness, ensuring deterministic sorted output matching Jekyll's behavior.
