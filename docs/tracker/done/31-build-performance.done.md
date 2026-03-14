# Issue 31: Build Performance Optimization

## Problem

Full site build takes ~16s in release mode (777 pages + 1454 static files). Page generation already uses `par_iter()` via rayon, but other phases are sequential.

## Requirements

- Profile the build to identify bottlenecks per phase (data loading, collection loading, context building, page generation, static file copying, sitemap/feed)
- Parallelize static file copying (1454 files currently copied sequentially in `static_files::copy_static_files`)
- Investigate parallel collection loading (collections are loaded one-by-one in a sequential loop in `main.rs` lines 142-148)
- Investigate whether sitemap/feed generation can overlap with static file copying
- Reduce unnecessary cloning in `main.rs` (see specific locations below)
- Target: measurable wall-clock improvement (document before/after times)
- All existing tests must continue to pass

## Scope

### In scope

1. **Parallel static file copying**: `static_files::copy_static_files` iterates files sequentially with `fs::copy`. Use rayon's `par_iter` to parallelize the copy loop. Directory creation needs care (ensure parent dirs exist before parallel copies, or use `create_dir_all` which is safe to call concurrently).

2. **Parallel collection loading**: In `main.rs`, collections are loaded in a sequential `for` loop. Each `load_collection` call is independent (reads different `_<name>/` directories). These can be loaded in parallel with rayon.

3. **Reduce unnecessary cloning**: The following locations in `main.rs` clone data that could be avoided:
   - Line 231: `items_to_build.into_iter().cloned().collect()` -- clones all `CollectionItem`s for each collection. Consider changing `generate_collection_pages` to accept `&[&CollectionItem]` instead of `&[CollectionItem]`.
   - Line 256: `pages_to_build.into_iter().cloned().collect()` -- same pattern for standalone pages.
   - Line 284: `posts_for_feed.into_iter().cloned().collect()` -- clones posts again for feed generation.
   - Line 273: `collections.into_iter().collect()` to convert HashMap to Vec -- could be avoided by iterating the HashMap directly.

4. **Phase timing**: Add per-phase timing output (behind a flag or always printed) so performance regressions can be detected.

### Out of scope

- Changing the template rendering engine
- Changing the markdown parser
- Async I/O (rayon thread-pool parallelism is sufficient for file I/O)
- Caching across builds (that is incremental builds, already implemented)

## Dependencies

- None. This is a standalone optimization issue.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (all existing tests still pass)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] Static file copying uses parallel iteration (rayon `par_iter` or similar)
- [ ] Collection loading is parallelized (all collections loaded concurrently)
- [ ] At least two of the unnecessary clones in `main.rs` are eliminated or reduced
- [ ] Build output includes per-phase timing (e.g., "Collections: 1.2s, Pages: 8.3s, Static files: 2.1s")
- [ ] Full site build (`datatalksclub.github.io`) shows measurable wall-clock improvement (before/after times documented)
- [ ] The generated `_site/` output is identical to the output before this change (diff the output directories to verify)

## Test Scenarios

### Unit: Parallel static file copying

- Copy 100+ files in a temp directory using the parallelized `copy_static_files`, verify all files are copied correctly with correct content
- Verify that nested directory structures are preserved when copying in parallel
- Verify the function still returns the correct count of copied files

### Unit: Parallel collection loading

- Load multiple collections in parallel, verify each collection has the expected number of items
- Verify that errors in one collection do not prevent other collections from loading

### Integration: Full site build correctness

- Build the `datatalksclub.github.io` site before and after the changes
- Diff the output directories (`_site/`) to verify identical output
- This is the most important test: performance optimizations must not change the output

### Performance: Timing verification

- Build the full site in release mode, verify that per-phase timings are printed
- Document the before/after total build time (should show improvement)

### Regression: Incremental builds still work

- Run a full build, then an incremental build, verify incremental still skips unchanged files
- The incremental manifest must still be written correctly

## Notes

- rayon is already a dependency (`rayon = "1.10"` in Cargo.toml)
- `create_dir_all` is safe to call concurrently (it is a no-op if the directory already exists)
- For parallel static file copying, collect all files first (already done by `collect_static_files`), then create all needed directories, then copy files in parallel
- For parallel collection loading, use `rayon::scope` or collect results from `par_iter` into a thread-safe structure
- The cloning issue at line 231/256 requires changing the signature of `generate_collection_pages` and `generate_standalone_pages` to accept references instead of owned values -- check if the downstream rayon `par_iter` in generator.rs can work with references
