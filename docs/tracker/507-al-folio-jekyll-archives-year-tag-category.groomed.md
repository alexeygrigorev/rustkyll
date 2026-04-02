# Issue 507: Support jekyll-archives-v2 per-collection config format (al-folio)

## Problem

al-folio uses `jekyll-archives-v2`, which has a per-collection config format that rustkyll does not parse. The current `ArchivesConfig::from_config()` looks for a top-level `enabled` key under `jekyll-archives`, but the v2 format nests config under collection names (`posts`, `books`). As a result, `from_config()` returns `None` and zero archive pages are generated for al-folio.

### Current al-folio config (v2 format)

```yaml
jekyll-archives:
  posts:
    enabled: [year, tags, categories]
    permalinks:
      year: "/blog/:year/"
      tags: "/blog/:type/:name/"
      categories: "/blog/:type/:name/"
  books:
    enabled: [year, tags, categories]
```

### Current rustkyll behavior (v1 format only)

```yaml
jekyll-archives:
  enabled: [year, tags, categories]   # <-- looks for this top-level key
  permalinks:
    year: "/:year/"
```

### Specific gaps identified

1. **V2 config parsing**: `ArchivesConfig::from_config()` does not detect per-collection keys (`posts:`, `books:`) as a v2 config. It returns `None`.

2. **`:type` permalink placeholder**: al-folio uses `"/blog/:type/:name/"` where `:type` resolves to `tag` or `category` (singular form). The current `resolve_permalink()` only handles `:name`.

3. **`page.documents` field**: al-folio's `archive.liquid` iterates `page.documents`, not `page.posts`. The current code only sets `page.posts` as an extra page field.

4. **`page.collection_name` field**: al-folio's layout uses `page.collection_name` (e.g., "posts", "books") which the current code does not set.

5. **`page.type` values (v2 uses plural)**: al-folio's layout checks `page.type == 'categories'` and `page.type == 'tags'` (plural). The v1 code sets `page.type` to `"category"` and `"tag"` (singular). The v2 format uses the plural form matching the config key names.

6. **`page.date` for year archives**: The layout uses `page.date | date: '%Y'` for year archive pages. The year archive page needs a synthetic `page.date` field set to e.g. `"2024-01-01"`.

7. **Multi-collection archive generation**: The current call site in `main.rs` only generates archives for `collections.get("posts")`. With v2, archives must be generated for each collection that has its own config block (`posts` AND `books`).

### Missing pages (39 archive pages)

**Blog archives (31 pages):**
- Year (7): `blog/2015/`, `blog/2020/`, `blog/2021/`, `blog/2022/`, `blog/2023/`, `blog/2024/`, `blog/2025/`
- Tag (21): `blog/tag/audios/`, `blog/tag/bib/`, `blog/tag/blockquotes/`, `blog/tag/charts/`, `blog/tag/citation/`, `blog/tag/code/`, `blog/tag/comments/`, `blog/tag/diagrams/`, `blog/tag/distill/`, `blog/tag/formatting/`, `blog/tag/google/`, `blog/tag/images/`, `blog/tag/jupyter/`, `blog/tag/links/`, `blog/tag/maps/`, `blog/tag/math/`, `blog/tag/medium/`, `blog/tag/sidebar/`, `blog/tag/tables/`, `blog/tag/toc/`, `blog/tag/videos/`
- Category (3): `blog/category/external-posts/`, `blog/category/external-services/`, `blog/category/sample-posts/`

**Books archives (8 pages):**
- Year (1): `books/2024/`
- Category (6): `books/category/classics/`, `books/category/crime/`, `books/category/historical-fiction/`, `books/category/mystery/`, `books/category/novels/`, `books/category/thriller/`
- Tag (1): `books/tag/top-100/`

## Baseline

- al-folio HTML files: 66/108 (rustkyll generates 66, Jekyll generates 108)
- al-folio archive pages generated: 0/39
- DTC DOM baseline: 596 matched / 790 total (must not regress)

## Scope

This issue covers:
- Parsing the jekyll-archives-v2 per-collection config format
- Generating archive pages for all configured collections (posts and books)
- Supporting the `:type` permalink placeholder
- Setting `page.documents` (alias for `page.posts`) in archive page context
- Setting `page.collection_name` in archive page context
- Using plural `page.type` values (`categories`, `tags`) for v2 configs
- Setting `page.date` on year archive pages

This issue does NOT cover:
- The 3 other missing al-folio pages (external posts, jupyter) -- those are separate issues
- Fixing DOM differences in existing al-folio pages
- Any changes to v1 jekyll-archives behavior (must remain backward-compatible)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (all existing tests plus new ones)
- [ ] V2 config is detected and parsed when `jekyll-archives` contains per-collection keys (e.g., `posts:`, `books:`)
- [ ] V1 config continues to work unchanged (backward compatibility)
- [ ] Archive pages are generated for EACH collection configured in v2 format
- [ ] `:type` placeholder in permalinks resolves to the archive type name (e.g., `tag`, `category`)
- [ ] `page.documents` is set on archive pages (same array as `page.posts`, for v2 template compatibility)
- [ ] `page.collection_name` is set on archive pages (e.g., `"posts"`, `"books"`)
- [ ] `page.type` uses plural form (`categories`, `tags`) for v2 configs
- [ ] `page.date` is set on year archive pages (e.g., `"2024-01-01 00:00:00 +0000"`)
- [ ] Building al-folio produces all 39 missing archive pages listed above
- [ ] al-folio HTML file count increases from 66/108 to at least 105/108
- [ ] DTC DOM match count does not drop below 596/790
- [ ] Generated archive pages contain correct post/document listings (verified by inspecting HTML output)
- [ ] Books collection archive pages are separate from blog archive pages (different URL prefixes)

## Test Scenarios

### Unit: V2 config parsing
- Parse a v2-format `jekyll-archives` YAML with `posts` and `books` collection keys; verify both collections are detected with correct enabled types and permalinks
- Parse a v2-format config with only one collection key; verify it works
- Parse a v1-format config (flat `enabled` + `permalinks`); verify backward compatibility -- v1 still parses correctly
- Parse a config that has neither `enabled` nor collection keys; verify `None` is returned
- Verify `:type` placeholder resolution: `resolve_permalink("/blog/:type/:name/", "code")` with type `tag` produces `/blog/tag/code/`

### Unit: page context fields
- Verify `page.documents` is populated on generated archive pages
- Verify `page.collection_name` is set correctly for each collection
- Verify `page.type` is `"categories"` (plural) for v2 category archives
- Verify `page.type` is `"tags"` (plural) for v2 tag archives
- Verify `page.date` is set on year archive pages with a parseable date string

### Integration: al-folio archive generation
- Build al-folio and verify year archive pages exist: `blog/2024/index.html`, `blog/2025/index.html`
- Build al-folio and verify tag archive pages exist: `blog/tag/code/index.html`, `blog/tag/formatting/index.html`
- Build al-folio and verify category archive pages exist: `blog/category/sample-posts/index.html`
- Build al-folio and verify books archive pages exist: `books/2024/index.html`, `books/category/classics/index.html`, `books/tag/top-100/index.html`
- Verify books archives are under `/books/` prefix, not `/blog/`
- Verify total al-folio HTML file count is at least 105/108

### Integration: backward compatibility
- Build a site that uses v1 jekyll-archives config (e.g., chirpy, academicpages, or a synthetic test site) and verify archive pages still generate correctly
- Run DTC DOM comparison and verify count does not drop below 596/790

### Output verification
- Inspect `blog/tag/code/index.html` HTML content: should contain links to posts tagged "code"
- Inspect `books/category/classics/index.html` HTML content: should contain links to books in "classics" category
- Inspect `blog/2024/index.html` HTML content: should contain links to posts from 2024

## Implementation Notes

### Config parsing approach

Detect v2 format by checking if the `jekyll-archives` mapping contains keys that are NOT standard v1 keys (`enabled`, `layouts`, `layout`, `permalinks`). If non-standard keys exist and they contain mappings with `enabled` arrays, treat as v2 format.

Alternatively, check for the `jekyll-archives-v2` plugin name in `config.plugins` to distinguish formats.

### Architecture suggestion

Create a new struct `ArchivesV2Config` (or extend `ArchivesConfig`) that holds a `HashMap<String, ArchivesConfig>` mapping collection names to their per-collection configs. The call site in `main.rs` should iterate over each collection in the v2 config and generate archive pages using the appropriate collection's items.

The v2 config for `books` has no explicit `permalinks`, meaning it should use defaults. Check what jekyll-archives-v2 defaults are -- likely `/:collection/:year/`, `/:collection/tag/:name/`, `/:collection/category/:name/`.

### Backward compatibility

The v1 code path must remain completely unchanged. The v2 detection should be a separate branch that only activates when the v2 format is detected. Existing sites using v1 must not be affected.

## Dependencies

- Issue #258 (jekyll-archives tag/category) -- DONE
- Issue #480 (jekyll-archives year/month/day) -- DONE
- Issue #505 (al-folio liquid layout extension support) -- IN PROGRESS (not a hard blocker; archive pages can generate even if layout rendering is imperfect)

## Log

### [PM] 2026-04-02 Grooming
- Read al-folio `_config.yml`: confirmed v2 per-collection format under `jekyll-archives`
- Built al-folio with rustkyll: 66/108 HTML files generated, 0/39 archive pages
- Confirmed `ArchivesConfig::from_config()` returns `None` for v2 format (no top-level `enabled` key)
- Identified 6 specific code gaps: v2 parsing, `:type` placeholder, `page.documents`, `page.collection_name`, plural `page.type`, `page.date`
- Verified DTC baseline: 596 matched / 790 total
- al-folio archive layout uses `page.documents` (not `page.posts`), `page.collection_name`, and plural `page.type` values
