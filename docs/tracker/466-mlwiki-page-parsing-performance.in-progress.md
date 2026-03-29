# Issue 466: mlwiki.org page parsing 2x slower than Jekyll

## Problem

mlwiki.org builds in 1.8s with rustkyll vs 0.98s with Jekyll.
The bottleneck is the Pages phase: 645 standalone pages at ~2.8ms each.
Page parsing (frontmatter extraction, markdown processing) has high
per-page overhead.

## Root Cause Analysis

Collection loading (`load_collection` in `src/collection.rs`) already uses a
two-phase approach: (1) collect file paths, then (2) `par_iter()` with rayon
to process files in parallel. This is why collections are fast.

`load_pages` (`src/collection.rs:982`) does NOT use this pattern. It calls
`load_pages_recursive` which does everything sequentially in a single pass:
file I/O (`fs::read_to_string`), frontmatter detection (`has_front_matter`),
YAML parsing (`frontmatter::parse_document`), markdown-to-HTML conversion
(`frontmatter::markdown_to_html_with_options`), URL computation, and
`Page` struct construction -- all in one serial loop per file.

For mlwiki's 645 pages, each doing markdown conversion at ~2.8ms, the serial
overhead adds up to ~1.8s. Parallelizing like collections do should bring
this well under Jekyll's 0.98s.

## Scope

1. Refactor `load_pages` into a two-phase approach matching `load_collection`:
   - Phase 1: Recursively collect candidate file paths (serial, fast)
   - Phase 2: Process files in parallel with `rayon::par_iter()` (file I/O,
     frontmatter parsing, markdown conversion, URL computation)
2. Keep the final sort step serial (it operates on the collected results)
3. Preserve all existing behavior: file filtering, directory skipping,
   published:false handling, README.md logic, permalink generation, etc.

## Dependencies

None. This is a self-contained performance optimization of `load_pages`.

## Baseline

- Build time: 1.8s (rustkyll) vs 0.98s (Jekyll). Target: < 0.98s.
- DTC DOM: 790/790
- mlwiki DOM: 576/644

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` is clean
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes (all existing tests, no regressions)
- [ ] `load_pages` uses rayon `par_iter()` for the file processing phase,
      matching the pattern already used by `load_collection`
- [ ] mlwiki.org total build time is under 0.98s (measured with `--verbose`
      or phase timing output; run 3 times and take the median)
- [ ] DTC DOM match count remains at 790/790
- [ ] mlwiki DOM match count remains at 576/644 or higher (no regression)
- [ ] All existing `test_load_pages_*` tests in `src/collection.rs` still pass
- [ ] Page ordering in `site.pages` is identical before and after the change
      (the sort at the end of `load_pages` must still produce the same order)

## Test Scenarios

### Unit: load_pages parallelization preserves results
- All existing `test_load_pages_*` tests pass unchanged (there are 20+
  tests covering count, index, exclusions, subdirectories, XML, HTML,
  README handling, etc.)
- No new unit tests needed for correctness since the refactor must be
  behavior-preserving; existing tests are the regression suite

### Integration: page order stability
- Build DTC site, capture the list of `page.url` values from `site.pages`
  context, verify order matches the pre-change order exactly
- Build mlwiki site, verify same page count (645 pages) and same page URLs

### Performance: mlwiki build time
- Build mlwiki.org 3 times with phase timing enabled
- Verify Pages phase time is under 0.5s (the parallelized phase)
- Verify total build time is under 0.98s

### Regression: DOM comparison
- Run DOM comparison for DTC: must stay at 790/790
- Run DOM comparison for mlwiki: must stay at 576/644 or higher

## Implementation Hints

Follow the exact pattern from `load_collection` (line ~611):
1. Create a `collect_page_paths` function (like `collect_collection_paths`)
   that does the recursive directory walk and filtering, returning `Vec<PathBuf>`
2. Use `.par_iter()` on the collected paths to do:
   - `fs::read_to_string`
   - `has_front_matter` check
   - `frontmatter::parse_document`
   - markdown conversion
   - URL computation
   - Return `Result<Page, CollectionError>`
3. Collect results, separate Ok/Err, sort pages, return

Key concerns:
- The `config` reference is `&SiteConfig` which is `Sync`, so it can be
  shared across rayon threads without issue
- `site_dir` is `&Path` which is also `Sync`
- The `add_code_classes`, `enable_hardbreaks`, `enable_autolink` booleans
  should be computed once before the parallel section (they are currently
  recomputed per file inside the loop -- another minor optimization)
