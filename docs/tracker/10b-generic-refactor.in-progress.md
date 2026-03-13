# Issue 10b: Refactor to Generic Jekyll Replacement

## Description

The generator currently has site-specific hardcoding that must be removed to make rustkyll a generic Jekyll replacement. Any Jekyll site should work without code changes. Also, the test suite must be FAST -- the whole point of Rust is speed.

## Dependencies

- Issue 09 (people pages) -- DONE
- Issue 10 (blog posts) -- DONE
- Issue 11 (books pages) -- must be DONE before this starts
- Issue 12 (podcast pages) -- must be DONE before this starts

## Scope

Five distinct changes, all in a single issue because they are tightly coupled:

1. Make LenientValue/LenientObject work for objects inside arrays (recursive leniency)
2. Remove hardcoded GitHub repo URL
3. Delete tests/ci_workflow.rs
4. Reorganize tests (integration tests to tests/ directory) and make the suite fast
5. Replace per-collection generator functions with a single generic one

---

## Problem 1: Hardcoded Field Normalization Constants

### Current State

`src/generator.rs` contains four hardcoded field lists:
- `EVENT_FIELDS` (13 fields)
- `TRANSCRIPT_ITEM_FIELDS` (5 fields)
- `CLIP_FIELDS` (4 fields)
- `PODCAST_FIELDS` (22 fields)

These exist because `LenientValue` in `src/template/engine.rs` only wraps `Object` values recursively -- when Liquid iterates over an `Array`, the individual array elements are returned as raw `Value` objects, losing their lenient behavior. So accessing a missing key on an object inside an array (e.g., `item.who` in a transcript loop) causes an "Unknown index" error.

The current workaround is `normalize_array_objects()` which pre-populates `Nil` for every expected field name on every object in an array. This is fragile and site-specific.

### Required Fix

Modify `LenientValue` in `src/template/engine.rs` so that:
- When `LenientValue` wraps a `Value::Array`, iterating over that array yields `LenientValue`-wrapped elements (not raw `Value`)
- This means objects inside arrays also return `Nil` for missing keys
- The recursion must work at ALL nesting levels: arrays of objects containing arrays of objects, etc.

The key technical challenge: the `liquid` crate's `ValueView` trait has an `as_array()` method that returns `Option<&dyn ArrayView>`. `LenientValue` must implement `ArrayView` for `Value::Array` cases, returning wrapped children. Each child element must itself be a `LenientValue`.

Once this works, DELETE entirely from `src/generator.rs`:
- `EVENT_FIELDS` constant
- `TRANSCRIPT_ITEM_FIELDS` constant
- `CLIP_FIELDS` constant
- `PODCAST_FIELDS` constant
- `normalize_array_objects()` function
- `normalize_podcast_front_matter()` function
- All call sites that invoke these functions

### How to Verify

- Render a podcast episode that has `transcript` items where some items lack `who` or `line` -- must not error
- Render a podcast episode that has `quotableClips` -- must not error
- Render events data where some events lack `youtube` or `anchor` -- must not error
- `grep -r "FIELDS\|normalize_array_objects\|normalize_podcast_front_matter" src/` returns nothing

---

## Problem 2: Hardcoded GitHub Repository URL

### Current State

In `src/generator.rs`, `build_site_context()` (line ~161) hardcodes:
```rust
LiquidValue::scalar("https://github.com/DataTalksClub/datatalksclub.github.io")
```

### Required Fix

Determine `site.github.repository_url` dynamically, in this priority order:
1. If `_config.yml` has a `repository` field (Jekyll convention, format: `owner/repo`), construct `https://github.com/{repository}`
2. Otherwise, read the git remote origin URL from the site directory (run `git remote get-url origin` or parse `.git/config`)
3. If neither is available, omit `site.github.repository_url` (set to Nil)

Add a `repository` field to `SiteConfig` (optional String). The DataTalks.Club `_config.yml` does not currently have one, so the fallback to git remote is important.

### How to Verify

- `grep -rn "DataTalksClub" src/` returns NO matches (the only reference should be in test fixtures or the datatalksclub.github.io subdir)
- Build the site and confirm `site.github.repository_url` resolves correctly in rendered HTML

---

## Problem 3: Delete Pointless CI Workflow Tests

### Current State

`tests/ci_workflow.rs` has 7 tests that parse `.github/workflows/ci.yml` and check its structure. This is pointless -- CI validates itself by running.

### Required Fix

Delete `tests/ci_workflow.rs`. Nothing else.

### How to Verify

- File does not exist: `tests/ci_workflow.rs`
- `cargo test` does not reference ci_workflow

---

## Problem 4: Test Organization and Speed

### Current State

- 345 tests in the lib crate, taking **198 seconds** (over 3 minutes)
- Many tests in `src/generator.rs` are full integration tests that load the entire site from disk, build layout engines, render pages, and verify HTML output
- Each such test independently loads collections, data files, and compiles templates -- massive redundant I/O and parsing
- Tests in `src/template/layout.rs`, `src/template/engine.rs`, etc. may also do heavy I/O

### Required Fix

#### Step 1: Separate integration tests

Move tests that touch the real `datatalksclub.github.io/` directory from inline `#[cfg(test)] mod tests` blocks into the `tests/` directory. Suggested structure:

```
tests/
  integration.rs          (keep existing CLI tests)
  integration_people.rs   (people page generation tests)
  integration_posts.rs    (post page generation tests)
  integration_books.rs    (book page generation tests)
  integration_podcast.rs  (podcast page generation tests)
  integration_context.rs  (site context building tests)
  integration_templates.rs (layout/template rendering with real site data)
```

Keep only true unit tests inline -- tests that use hardcoded strings, mock data, or small fixtures. A test is an integration test if it calls `site_dir()`, `load_collection()` with the real site, `load_data()` with the real `_data/` dir, or `LayoutEngine::new()` with real layouts.

#### Step 2: Make tests fast

The target is `cargo test` completes in under 15 seconds for the FULL suite. Strategies:

1. **Share expensive setup across tests**: Use `std::sync::LazyLock` (or `once_cell::sync::Lazy`) to load collections, data, config, and layout engine ONCE per test binary, shared across all tests in that file. This is the single biggest win -- right now each test independently loads ~700 files from disk.

2. **Parallelize page generation**: Add `rayon` as a dependency. Use `par_iter()` in `generate_collection_pages()` for rendering items in parallel. This helps both tests and actual site builds.

3. **Cache template compilation**: If the `LayoutEngine` recompiles templates on every call, cache the compiled templates.

4. **Reduce redundant assertions**: Some test files have dozens of per-item tests (e.g., one test per person) that each load the full site. Consolidate these into single tests that iterate.

#### Step 3: Build speed target

Full site generation (all collections) must complete in under 5 seconds. This is verified by a dedicated integration test with timing assertions.

### How to Verify

- `cargo test` completes in under 15 seconds
- `cargo test` still reports the same number of passing tests (or more) -- no test coverage is lost
- `src/generator.rs` inline tests are only unit tests (no `site_dir()` references)
- Integration tests in `tests/` cover all the scenarios that were previously inline

---

## Problem 5: Single Generic Collection Generator

### Current State

Four separate functions:
- `generate_people_pages()` (line 333) -- loads people, posts, books, data; builds context; calls `generate_collection_pages()`
- `generate_posts()` (line 455) -- loads posts, people, books, data; builds its own layout engine; manual loop
- `generate_book_pages()` (line 537) -- loads books, people, posts, data; builds context; calls `generate_collection_pages()`
- `generate_podcast_pages()` (line 650) -- loads podcast, people, posts, books, data; builds context with podcast array; manual loop with `normalize_podcast_front_matter()`

These are 90% identical -- they all load collections, build a site context, and render pages. The differences are:
- Which collections go into `site.*` (e.g., podcast adds `site.podcast`)
- Post output paths use `/blog/<slug>.html` instead of `/<collection>/<slug>.html`
- Podcast does front matter normalization (which Problem 1 eliminates)

### Required Fix

Keep the existing `generate_collection_pages()` function (line 275) as the core loop, but replace the four convenience functions with a single higher-level function:

```rust
pub fn generate_site_collection(
    collection_name: &str,
    site_dir: &Path,
    config: &SiteConfig,
    layout_engine: &LayoutEngine,
    site_context: &Object,
    output_dir: &Path,
) -> Result<GenerationResult, GeneratorError>
```

Or alternatively, a `SiteBuilder` struct that holds the shared state (config, layout engine, site context, loaded collections) and has a single `generate_collection(&mut self, name: &str)` method.

The key constraint: NO collection-type-specific logic in the generation code. The layout comes from config defaults or front matter. The permalink pattern comes from the collection config. The output path is computed from the permalink. No `if collection == "podcast" { ... }` branches.

Functions that can stay (they are legitimate helpers, not collection-specific):
- `build_site_context()` -- but it should be generic (accept a list of collection names to populate, not hardcoded posts/books)
- `collection_item_to_liquid()` -- generic conversion
- `resolve_layout()` -- already generic
- `output_path()` -- already generic

Functions to DELETE or fold into the generic path:
- `generate_people_pages()`
- `generate_posts()`
- `generate_book_pages()`
- `generate_podcast_pages()`
- `normalize_podcast_front_matter()` (already deleted by Problem 1)
- `normalize_array_objects()` (already deleted by Problem 1)
- `build_podcast_site_context()` -- fold into generic `build_site_context()`
- `build_post_site_context()` -- fold into generic `build_site_context()`
- `build_post_front_matter()` -- fold into generic path
- `post_output_path()` -- compute from permalink pattern instead

`build_site_context()` should accept a map of all loaded collections and populate `site.<collection_name>` for each. No special-casing.

### How to Verify

- `grep -n "fn generate_people_pages\|fn generate_posts\|fn generate_book_pages\|fn generate_podcast_pages" src/generator.rs` returns nothing
- `grep -n "fn build_podcast_site_context\|fn build_post_site_context\|fn normalize_podcast_front_matter\|fn normalize_array_objects" src/generator.rs` returns nothing
- A single generic function (or method) generates all collections
- All existing integration tests still pass with the new generic code path

---

## Acceptance Criteria

- [ ] `cargo build` compiles without errors or warnings (`cargo clippy -- -D warnings` clean)
- [ ] No hardcoded field name constants anywhere in `src/` -- `grep -rn "EVENT_FIELDS\|PODCAST_FIELDS\|TRANSCRIPT_ITEM_FIELDS\|CLIP_FIELDS" src/` returns nothing
- [ ] No `normalize_array_objects` or `normalize_podcast_front_matter` functions in `src/`
- [ ] `LenientValue` in `src/template/engine.rs` handles missing keys on objects inside arrays at all nesting levels
- [ ] No hardcoded URLs -- `grep -rn "DataTalksClub" src/` returns nothing (only allowed in test fixtures and the datatalksclub.github.io directory)
- [ ] `site.github.repository_url` is derived from config or git remote, not hardcoded
- [ ] `tests/ci_workflow.rs` does not exist
- [ ] Per-collection generator functions (`generate_people_pages`, `generate_posts`, `generate_book_pages`, `generate_podcast_pages`) do not exist -- replaced by a single generic function
- [ ] `cargo test` full suite (all unit + integration tests) completes in under 15 seconds
- [ ] Full site build (all collections rendered to HTML) completes in under 5 seconds
- [ ] All previously passing tests still pass -- no test coverage regression
- [ ] Integration tests live in `tests/` directory, not inline in `src/`
- [ ] Unit tests in `src/` do not reference `site_dir()` or load the real datatalksclub.github.io site

---

## Test Scenarios

### Unit: LenientValue recursive leniency
- Create a LenientValue wrapping an array of objects where some objects lack certain keys; access a missing key and verify Nil is returned (not an error)
- Create a LenientValue wrapping an array of objects containing nested arrays of objects; access a missing key two levels deep and verify Nil
- Create a LenientValue wrapping an object containing an array; iterate the array in a Liquid template and access a missing field; verify renders as empty string

### Unit: GitHub repository URL resolution
- Config with `repository: "owner/repo"` produces `https://github.com/owner/repo`
- Config without `repository` but with a git remote origin produces the correct URL
- Config without `repository` and no git remote produces Nil for `site.github.repository_url`

### Unit: Generic collection generation
- Generate pages for a mock collection with 2 items using the generic function; verify both HTML files are created
- Verify layout resolution falls through from front matter to config defaults
- Verify output path is computed correctly from permalink pattern for different collection types

### Integration: Full site generation
- Generate all people pages using the generic path; verify count matches previous implementation (427+)
- Generate all posts using the generic path; verify count matches previous implementation (55+)
- Generate all book pages using the generic path; verify count matches previous implementation (99+)
- Generate all podcast pages using the generic path; verify count matches previous implementation (196+)
- Verify specific rendered HTML output matches expected content (spot-check at least one page per collection)

### Integration: Podcast with nested arrays
- Render a podcast episode with transcript items (some missing `who`, some missing `line`); verify no errors and output contains expected content
- Render a podcast episode with `quotableClips`; verify no errors

### Performance: Test suite timing
- `cargo test` completes in under 15 seconds (assert wall clock time)
- Full site build completes in under 5 seconds (assert wall clock time in an integration test)

---

## Implementation Notes

### LenientValue Array Support -- Technical Approach

The `liquid` crate's `ValueView` trait requires implementing `as_array() -> Option<&dyn ArrayView>` for array wrapping to work. The `ArrayView` trait requires methods like `values()`, `contains_key()`, `get()`, `size()`.

The `LenientValue` struct already stores `children` for object wrapping. For arrays, it needs an additional field (e.g., `array_children: Vec<LenientValue>`) that stores pre-wrapped array elements. When `from_value()` receives a `Value::Array`, it recursively wraps each element.

### Shared Test Fixtures

Use `std::sync::LazyLock` (stable in Rust 1.80+) for shared test state:

```rust
static SITE_CONFIG: LazyLock<SiteConfig> = LazyLock::new(|| {
    SiteConfig::from_file(&site_dir().join("_config.yml")).unwrap()
});

static PEOPLE: LazyLock<Vec<CollectionItem>> = LazyLock::new(|| {
    let (items, _) = load_collection("people", &site_dir(), &SITE_CONFIG).unwrap();
    items
});
```

This ensures collections are loaded once across all tests in a file, cutting I/O from ~345x to ~1x.
