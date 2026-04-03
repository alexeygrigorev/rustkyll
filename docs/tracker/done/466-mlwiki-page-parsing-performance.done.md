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
- DTC DOM: 596/790 matched
- mlwiki DOM: 535/644 matched

## Acceptance Criteria

- [x] `cargo build` compiles without errors
- [x] `cargo clippy -- -D warnings` is clean
- [x] `cargo fmt` produces no changes
- [x] `cargo test` passes (all existing tests, no regressions)
- [x] `load_pages` uses rayon `par_iter()` for the file processing phase,
      matching the pattern already used by `load_collection`
- [x] mlwiki.org total build time is under 0.98s on warm runs with a stable
      destination (three runs: 0.72s, 0.73s, 0.74s; median 0.73s)
- [x] DTC DOM match count remains at 596/790 matched (no regression from
      current committed baseline)
- [x] mlwiki DOM match count remains at 535/644 matched (no regression from
      current committed baseline; previous 576 baseline was stale)
- [x] All existing `test_load_pages_*` tests in `src/collection.rs` still pass
- [x] Page ordering in `site.pages` is identical before and after the change
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
- Build mlwiki.org 3 times with stable warm-cache methodology
- Verify Pages phase time is under 0.5s (the parallelized phase)
- Verify total build time is under 0.98s

### Regression: DOM comparison
- Run DOM comparison for DTC: must stay at 596/790 matched or higher
- Run DOM comparison for mlwiki: must stay at 535/644 matched or higher

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

## Log

### [SWE] 2026-04-03 11:30
- Refactored `load_pages` in `src/collection.rs` to match the two-phase collection loader pattern.
- Added `collect_page_paths` for serial discovery and `process_page_file` for parallel file parsing with `rayon::par_iter()`.
- Preserved the final `site.pages` sort and existing page filtering, README handling, published:false handling, permalink generation, and non-markdown front matter behavior.
- Verification:
  - `cargo test -p rustkyll test_load_pages_ -- --nocapture` passed
  - `cargo test -p rustkyll` passed
  - `cargo clippy -p rustkyll -- -D warnings` passed
  - `cargo fmt --check` passed after formatting the repo
- Performance:
  - warm-run builds using a stable destination: `0.72s`, `0.73s`, `0.74s`
  - median: `0.73s`
- DOM verification:
  - DTC compare remained at `596/790` matched
  - mlwiki compare remained at `535/644` matched
  - verified that `535/644` is the actual current `HEAD` result by building a clean worktree at commit `4ab9d7d`; the older `576/644` issue baseline was stale and not caused by this change
- Files modified: `src/collection.rs`, `docs/dom-baselines.json`

### [QA] 2026-04-03 11:40
- `cargo test -p rustkyll`: PASS
- `cargo clippy -p rustkyll -- -D warnings`: PASS
- `cargo fmt --check`: PASS
- `test_load_pages_*` regression suite: PASS
- Performance target: PASS on warm runs with stable destination (`0.73s` median < `0.98s`)
- DOM baseline: PASS
  - DTC `596/790` matched, no regression from committed baseline
  - mlwiki `535/644` matched, no regression from committed baseline
- Note: the previous `576/644` mlwiki baseline was stale. Clean `HEAD` reproduces `535/644`, so this patch does not introduce a DOM regression.
- VERDICT: PASS

### [PM] 2026-04-03 11:45
- Reviewed the refactor and validation results.
- Acceptance criteria met with current verified baselines.
- `load_pages` now follows the same scalable pattern as collection loading without changing observed output.
- The DOM-baseline correction for mlwiki is justified by an independent clean-`HEAD` rebuild.
- VERDICT: ACCEPT
