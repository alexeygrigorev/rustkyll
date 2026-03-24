# Issue 328: academicpages -- push from 10/45 to 40+/45

## Problem

academicpages (13k GitHub stars, popular academic portfolio theme) currently matches 10/45 pages. It was first analyzed in issue 326, which got it from 1/45 to 10/45 by fixing `include:` config, YAML merge keys, and other foundational issues. The remaining 35 pages with diffs break down into clear categories, most of which are high-leverage.

### Diff Breakdown (35 pages with diffs)

**Category A: Date-only diffs (14 pages) -- trivial, cache regeneration**

14 pages differ ONLY in "Site last updated 2026-03-20" vs "2026-03-23". These are:
- 404.html, cv-json/index.html, index.html, non-menu-page/index.html
- publication/2009-10-01-paper-title-number-1.html, publication/2010-10-01-paper-title-number-2.html, publication/2015-10-01-paper-title-number-3.html
- talkmap.html, talks/2012-03-01-talk-1.html, talks/2013-03-01-tutorial-1.html, talks/2014-02-01-talk-2.html, talks/2014-03-01-talk-3.html
- teaching/2014-spring-teaching-1.html, teaching/2015-spring-teaching-1.html

Fix: Regenerate Jekyll cached output.

**Category B: Blog posts rendering without layout (4 pages, ~16 diffs)**

Posts output as raw HTML fragments without `<html>`, `<head>`, `<body>` wrappers:
- posts/2012/08/blog-post-1/index.html (4 diffs)
- posts/2012/08/blog-post-4/index.html (2 diffs)
- posts/2013/08/blog-post-2/index.html (4 diffs)
- posts/2014/08/blog-post-3/index.html (4 diffs)

Root cause: The `defaults:` config specifies `type: posts` with `layout: single`, but this layout is not being applied. The posts render as just their markdown content converted to HTML. This is a defaults-for-posts resolution bug -- the `type: posts` default scope is not matching collection items.

**Category C: Raw Liquid in output -- extensionless includes (2 pages, ~8 diffs)**

Two pages have completely unprocessed Liquid tags in the output:
- categories/index.html (4 diffs) -- raw `{% include group-by-array %}`, `{% for %}` tags
- tags/index.html (4 diffs) -- same pattern

Root cause: Both pages use `{% include group-by-array collection=... %}`. The file `_includes/group-by-array` has NO file extension. Jekyll finds includes without extensions, but rustkyll likely requires `.html` extension to locate the include file. When the include fails, the entire Liquid template renders as raw text.

**Category D: SEO tag element ordering (2 pages, ~48 diffs)**

portfolio/portfolio-1/index.html and portfolio/portfolio-2/index.html have 24 diffs each. All diffs are SEO meta tag ordering -- `<link>`, `<script>`, `<meta>` elements appear in different order in the `<head>`. The content is correct but the ordering differs from Jekyll's jekyll-seo-tag output.

**Category E: Page ordering in archive pages (2 pages, ~139 diffs)**

page-archive/index.html (53 diffs) and sitemap/index.html (86 diffs) list pages in different order. Jekyll sorts pages alphabetically by URL. Rustkyll appears to use a different ordering, causing every page entry to show as a diff even though all pages are present.

**Category F: `{:toc}` kramdown TOC directive (2 pages, ~8 diffs)**

markdown/index.html and terms/index.html contain `{:toc}` (kramdown's table-of-contents directive). Rustkyll outputs `* Auto generated table of contents` as raw text instead of generating the actual TOC HTML. The `toc__menu` class is being added to the header but no `<ul>` tree is generated.

**Category G: Collection archive pages -- missing excerpts/content (5 pages, ~50 diffs)**

collection-archive/index.html (18 diffs), portfolio/index.html (11 diffs), talks/index.html (5 diffs), teaching/index.html (3 diffs), year-archive/index.html (31 diffs) -- archive pages that iterate over collections show incorrect excerpt rendering: `class='page__date'` instead of `class='page__meta'`, `<strong>` instead of `<i>`, extra `<time>` elements, missing `<p>` elements for descriptions.

Root cause: The `archive-single.html` include uses `page__meta` class with `<i>` icon elements for read time and date. Rustkyll either uses a different include or renders the metadata differently.

**Category H: Math notation in front matter (3 pages, ~13 diffs)**

publication/2024-02-17-paper-title-number-4.html (10 diffs), publications/index.html (3 diffs), cv/index.html (1 diff beyond date) -- front matter contains `$$E=mc^2$$` which Jekyll converts to `\(E=mc^2\)` in titles/descriptions. Rustkyll preserves the raw `$$..$$` delimiters.

**Category I: Markdown rendering diffs (1 page, 131+ diffs)**

archive-layout-with-content/index.html has complex markdown content with tables, definition lists, and other elements that differ structurally.

## Scope

Priority order by impact (page count gained):

1. **Category A** (14 pages) -- Regenerate Jekyll cached output.
2. **Category B** (4 pages) -- Fix defaults application for `type: posts` collection items. This is likely a bug where the defaults matching logic does not associate `type: posts` with collection items from the `_posts` directory.
3. **Category C** (2 pages) -- Support extensionless include files. When `{% include name %}` is used and `_includes/name` exists (without `.html`), resolve it.
4. **Category E** (2 pages) -- Fix page ordering in `site.pages` to match Jekyll's alphabetical-by-URL ordering.
5. **Category G** (5 pages) -- Fix archive-single include rendering for collection items (excerpt, read time, metadata classes).
6. **Category H** (3 pages) -- Convert `$$..$$` to `\(..\)` in front matter values used in titles/meta tags.
7. **Category D** (2 pages) -- Fix SEO tag element ordering in `<head>`.
8. **Category F** (2 pages) -- Implement `{:toc}` directive. May be descoped if complex.
9. **Category I** (1 page) -- Complex markdown diffs. May be descoped.

Categories A-E are required. Categories F-I may be descoped to follow-up issues with explicit documentation.

## Dependencies

- Issue 326 (benchmark chirpy/academicpages/doctheme) -- DONE. This continues that work.
- Issue 327 (documentation-theme-jekyll) -- in progress, no conflict.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `./scripts/cargo-safe test` passes with all existing tests plus new tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] academicpages DOM match reaches 40+/45 (up from 10)
- [ ] Blog posts render with full layout (html/head/body structure)
- [ ] `{% include group-by-array %}` resolves correctly (extensionless include file found)
- [ ] categories/index.html and tags/index.html render with processed Liquid (no raw `{% %}` tags)
- [ ] Page ordering in archive pages matches Jekyll's ordering
- [ ] If 45/45 is not achieved, the engineer must document every remaining diff category and either fix it or create a follow-up issue
- [ ] No regressions on DTC (must remain 751+/790)
- [ ] No regressions on muan-blog (must remain 2172+/2218)
- [ ] No regressions on mlwiki (must remain 560+/644)
- [ ] No regressions on sites currently at 100% (lanyon, minima, choosealicense, etc.)
- [ ] Tests include non-ASCII/Unicode content (math notation `$$E=mc^2$$`, CJK if applicable)
- [ ] At least 10 new test functions covering the fixes

## Test Scenarios

### Unit: Extensionless include resolution

- Create `_includes/my-include` (no extension) containing `<p>included</p>`
- Render `{% include my-include %}` in a template
- Verify output contains `<p>included</p>`
- Verify `{% include my-include.html %}` still resolves `_includes/my-include.html` (existing behavior preserved)

### Unit: Extensionless include with parameters

- Create `_includes/group-helper` containing `{{ include.field }}`
- Render `{% include group-helper field="categories" %}`
- Verify output contains `categories`

### Unit: Defaults matching for type: posts

- Configure `defaults:` with `type: posts`, `values: { layout: single }`
- Process a post from `_posts/2024-01-01-test.md`
- Verify the post receives `layout: single` from defaults
- Verify the layout is applied (output contains full HTML structure)

### Unit: Defaults matching for type: pages

- Configure `defaults:` with `type: pages`, `values: { layout: single }`
- Process a page from `_pages/about.md` (with `include: ["_pages"]` config)
- Verify the page receives `layout: single` from defaults

### Unit: Page ordering in site.pages

- Create 5 pages with URLs: /cv/, /about/, /markdown/, /404.html, /archive/
- Render a template that iterates `{% for page in site.pages %}{{ page.url }},{% endfor %}`
- Verify pages are output in alphabetical URL order matching Jekyll

### Unit: Math notation conversion in front matter

- Front matter: `title: "Test with $$E=mc^2$$ formula"`
- Verify title in rendered output uses `\(E=mc^2\)` notation (matching Jekyll's mathjax processing)
- Include non-ASCII: `title: "Formule $$\alpha + \beta$$"`

### Unit: Unicode in extensionless includes

- Create `_includes/unicode-test` containing non-ASCII content
- Verify the include resolves and content is preserved

### Integration: academicpages full build and DOM comparison

- Build academicpages with rustkyll
- Run DOM comparison against Jekyll cached output (after cache regeneration)
- Verify 40+ pages match
- Spot-check previously-failing pages:
  - `categories/index.html` -- must have processed Liquid, actual category groupings
  - `tags/index.html` -- must have processed Liquid, actual tag groupings
  - `posts/2012/08/blog-post-1/index.html` -- must have full HTML layout
  - `page-archive/index.html` -- pages listed in correct alphabetical order
  - `publication/2024-02-17-paper-title-number-4.html` -- math notation converted

### Regression: Other sites

- Run `./scripts/cargo-safe test` full suite
- Verify DTC remains 751+/790
- Verify no regression on any currently-passing site

## Output Verification

```bash
# Step 1: Regenerate Jekyll cache
cd websites/academicpages
bundle exec jekyll build --destination _site_jekyll_cached
cd ../..

# Step 2: Build with rustkyll
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/academicpages \
  --destination /tmp/academic_328

# Step 3: DOM comparison
uv run scripts/dom_compare.py \
  --jekyll-dir websites/academicpages/_site_jekyll_cached \
  --rustkyll-dir /tmp/academic_328
```

Expected: 40+ files matched (up from 10).

Spot-checks:
```bash
# Categories page must not have raw Liquid
grep '{%' /tmp/academic_328/categories/index.html
# Expected: 0 lines

# Blog posts must have full HTML structure
head -5 /tmp/academic_328/posts/2012/08/blog-post-1/index.html
# Expected: <!DOCTYPE html> or <html>

# Page archive must list pages in alphabetical order
grep 'href=' /tmp/academic_328/page-archive/index.html | head -5
# Expected: 404.html comes first
```

## Notes

- The 14 date-only diffs require regenerating Jekyll cached output. If the engineer cannot run Jekyll locally, document this and the PM will arrange cache regeneration. Pages with only date diffs should still count toward the acceptance target after cache regeneration.
- The extensionless include (`group-by-array`) is a legitimate Jekyll feature. Some themes use it intentionally. The fix should be generic (check for the file as-is when no extension match found), not academicpages-specific.
- The blog post layout issue may affect other sites that rely on `type: posts` in defaults. The fix should be tested against DTC and other sites to verify no regression.

## Log

### [SWE] 2026-03-23

**Analysis:** Investigated all 35 pages with diffs. Found root causes:
- Blog posts rendering without layout: NOT a defaults resolution bug. The `type: posts` defaults were correctly applied. The actual failure was in layout rendering -- the `tag-list.html` include contained unsupported Liquid syntax (`{% for x in arr | filter %}` and `{% assign x = (expr) %}`), causing the entire layout chain to fail and fall back to raw HTML.
- Extensionless includes: Actually worked correctly (loaded by `load_includes`). The issue was include files containing unsupported Liquid syntax that only templates (not includes) got preprocessed for.
- Page ordering: Sort key was wrong -- needed filename-first sort, not path sort.
- Date diffs: Regenerated Jekyll cache.

**Fixes implemented:**

1. **`preprocess_for_loop_filters`** (engine.rs) -- Extracts filter chains from `{% for var in expr | filter %}` into a temporary assign + plain for. Jekyll supports filters in for-loop iterables; the Liquid crate does not.
   - TDD: Test written first (4 tests), verified failure, implemented, verified pass.

2. **`preprocess_parenthesized_assign`** (engine.rs) -- Strips parentheses from `{% assign var = (expr | filter) %}`. Jekyll allows this syntax; Liquid crate does not.
   - TDD: Test written first (3 tests), verified failure, implemented, verified pass.

3. **Full preprocessing pipeline for include files** (engine.rs `build_partials`) -- Previously only `preprocess_include_paths` was applied to include file content. Now all preprocessing steps (capture tags, Jekyll tags, nil contains, nil eq false, nested braces, for-loop filters, parenthesized assign) are applied. This fixed the root cause of blog posts failing to render.

4. **Hyphenated include name quoting** (include_tag.rs) -- Extended `preprocess_include_paths` to quote include filenames containing hyphens (e.g., `group-by-array`). The Liquid tokenizer treats `-` as the minus operator, breaking unquoted names.

5. **Page ordering fix** (collection.rs) -- Changed `load_pages` sort from `(basename, url)` to `(filename, source_path)` matching Jekyll's behavior where files from different directories interleave by filename.

6. **Custom `join` filter** (join.rs) -- Created lenient join that:
   - Handles string input by returning as-is (Jekyll behavior)
   - Recursively flattens nested arrays (Ruby `Array#join` behavior)
   - Required for `group-by-array` include's `join: ',' | join: ','` flatten pattern

7. **Math notation in markdownify** (markdownify.rs) -- Converts `$$..$$` to `\(..\)` in markdownify filter output, matching kramdown's inline math notation.

8. **Sort filter fallback** (sort.rs) -- When `sort:N` property lookup returns nil for all items (e.g., sorting strings by non-existent property), fall back to value sort instead of preserving original order.

9. **README.md discovery fix** (collection.rs) -- README.md files without front matter are now only included if there's a path-specific default targeting them, not just a catch-all `type: pages` default. Prevents `markdown_generator/README.md` from being treated as a page.

10. **Jekyll cache regeneration** -- Ran `bundle exec jekyll build` to update the cached output with current date, fixing 14 date-only diffs.

11. **Pre-existing clippy fix** (main.rs) -- Removed unnecessary `mut` on `site_context`.

**Test results:** 2722 lib + 41 bin + other = ~2800 total tests pass. 0 failures. Clippy clean. Fmt clean.

**DOM comparison result:** 27/45 matched (up from 10/45).

**Files modified:**
- `src/template/engine.rs` -- `preprocess_for_loop_filters`, `preprocess_parenthesized_assign`, full preprocessing in `build_partials`
- `src/template/include_tag.rs` -- Hyphen quoting in `preprocess_include_paths`
- `src/template/filters/join.rs` -- NEW: lenient join filter with recursive flatten
- `src/template/filters/mod.rs` -- Register join filter
- `src/template/filters/markdownify.rs` -- Math notation conversion
- `src/template/filters/sort.rs` -- Nil property fallback to value sort
- `src/collection.rs` -- Page sort, README.md discovery
- `src/main.rs` -- Pre-existing clippy fix
- `src/frontmatter.rs` -- (reverted, no net changes)
- `websites/academicpages/_site_jekyll_cached/` -- Regenerated

**Remaining diffs by category (18 pages):**

| Category | Pages | Issue | Suggested Follow-up |
|----------|-------|-------|---------------------|
| D: SEO ordering | 2 (portfolio-1, portfolio-2) | 23 diffs each: meta tag ordering in head | Follow-up issue |
| E: Page archive | 2 (page-archive, sitemap) | 13+46 diffs: missing redirect pages (jekyll-redirect-from plugin) | Follow-up issue: redirect support |
| F: {:toc} | 2 (markdown, terms) | 135+3 diffs: kramdown TOC directive not implemented | Follow-up issue |
| G: Archive rendering | 4 (collection-archive, talks, teaching, year-archive) | 2-30 diffs: excerpt/metadata class diffs in archive-single include | Follow-up issue |
| H: Math in body | 1 (publication) | 2 diffs: $$...$$ in body code blocks, smart quote in code | Follow-up issue |
| I: Complex markdown | 1 (archive-layout-with-content) | 130 diffs: tables, definition lists | Follow-up issue |
| B': Blog post related | 3 (blog-post-1,2,3) | 1 diff each: missing related_posts div | Investigate page.related truthiness |
| B': URL collision | 1 (blog-post-4) | 25 diffs: wrong post renders due to URL collision | Follow-up issue: collision resolution |
| Tags rendering | 1 (tags/index.html) | 90 diffs: archive-single template diffs | Same as G |
