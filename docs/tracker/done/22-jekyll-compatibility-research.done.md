# Issue 22: Jekyll Compatibility Research -- Report

## Test Websites

The following open-source Jekyll sites were cloned into `websites/` (gitignored) for testing.

### Setup

```bash
mkdir -p websites
cd websites
git clone --depth 1 https://github.com/jekyll/minima.git
git clone --depth 1 https://github.com/daattali/beautiful-jekyll.git
git clone --depth 1 https://github.com/mmistakes/minimal-mistakes.git
git clone --depth 1 https://github.com/github/choosealicense.com.git
```

### Repos and Commit Hashes

| Site | Repo | Commit | Description |
|------|------|--------|-------------|
| minima | `jekyll/minima` | `bf9ef989246b63536e9db61082f663f1a6d4d9ce` | Jekyll's default theme |
| beautiful-jekyll | `daattali/beautiful-jekyll` | `476d4a4144bdc62cc57f5d69f72903d299a7c0df` | Popular Jekyll theme with demo site |
| minimal-mistakes | `mmistakes/minimal-mistakes` | `5786c35342d4f359ca40479f38e6255785ab1591` | Feature-rich Jekyll theme |
| choosealicense.com | `github/choosealicense.com` | `b4442b34d1ce233b2bef5855181d77c61575afef` | Real GitHub Pages site |

## Build Results

### Baseline: DataTalks.Club site

```
Build complete!
  Collection pages: 767
  Standalone pages: 10
  Total pages:      777
  Sitemap entries:  787
  Static files:     1454
  Time:             67.27s
  Warnings: 8
```

The 8 warnings are:
- 4 posts fail due to include paths with slashes (e.g., `course-structured-data/mlops-zoomcamp-structured-data.html`) -- the Liquid parser rejects `/` in include names
- 4 posts fail due to `include["max_posts"]` accessing parameters from include calls -- `include` variable scope not fully supported

### Test Site: minima

**Result: BUILD FAILED** -- `config error: failed to parse config YAML: missing field 'url' at line 93 column 1`

The minima `_config.yml` is entirely comments with no actual values set. Jekyll provides defaults for missing fields (`url`, `name`, `title`). Rustkyll requires these fields, failing on startup.

### Test Site: beautiful-jekyll

**Result: BUILD FAILED** -- `config error: failed to parse config YAML: missing field 'url' at line 13 column 1`

This site has `title` and `author` (as string) but no `url` or `name` field. Jekyll treats missing `url` as `""` and `name` as optional.

### Test Site: minimal-mistakes

**Result: BUILD FAILED** -- `config error: failed to parse config YAML: twitter: invalid type: map, expected a string at line 93 column 3`

This site uses `twitter` as a map (`twitter: { username: ... }`) rather than a string. Rustkyll's `SiteConfig` defines `twitter: Option<String>`, which cannot deserialize a map.

### Test Site: choosealicense.com

**Result: BUILD FAILED** -- `config error: failed to parse config YAML: twitter: invalid type: map, expected a string at line 47 column 3`

Same `twitter` map issue. This site also has `collections` with `:path/` permalink (rather than `:title.html`), `redirect-from` plugin usage, and `jekyll-seo-tag`.

## Feature Gap Analysis

### CRITICAL -- Blocks all external Jekyll sites from building

#### 1. Rigid Config Parsing

**Problem:** Rustkyll requires `url`, `name`, and `title` as mandatory string fields. Real Jekyll sites:
- Often omit `url` (Jekyll defaults to `""`)
- Often omit `name` (Jekyll treats it as optional)
- Use `twitter` as a map (`{username: "@foo"}`) not a string

**Used by:** ALL 4 test sites, plus the majority of Jekyll sites in the wild.

**Fix:** Make `url`, `name` optional with defaults. Change `twitter` to `Option<serde_yaml::Value>` to accept both strings and maps. Use `#[serde(flatten)]` or a catch-all for unknown config keys.

**Complexity:** Low (config struct changes only).

#### 2. Unknown Config Keys Cause Errors

**Problem:** Rustkyll's `SiteConfig` uses strict deserialization. Any config key not defined in the struct (e.g., `baseurl`, `locale`, `minimal_mistakes_skin`, `sass`, `kramdown`, `highlighter`, `paginate`, `timezone`, `markdown`, `compress_html`, `plugins`) causes a parse error or is silently ignored depending on serde behavior.

**Used by:** ALL external sites have many config keys not in rustkyll's schema.

**Fix:** The struct should use `#[serde(deny_unknown_fields)]` or (better) allow unknown fields and provide access to arbitrary config values via `site.config["key"]`.

**Complexity:** Low-Medium.

### HIGH PRIORITY -- Common features missing

#### 3. Pagination (`jekyll-paginate`)

**Problem:** No support for `paginator` object or paginated pages. The `paginate` and `paginate_path` config options are ignored.

**Used by:** beautiful-jekyll, minimal-mistakes, minima (3/4 test sites). Extremely common in blog sites.

**What it does:** Splits post lists into pages of N posts each. Provides `paginator.posts`, `paginator.total_pages`, `paginator.previous_page`, `paginator.next_page`, etc.

**Complexity:** Medium. Requires generating multiple index pages and populating a `paginator` context object.

#### 4. Sass/SCSS Compilation

**Problem:** Jekyll compiles `.scss` files in `_sass/` directory and any `.scss` file with front matter in `assets/`. Rustkyll copies CSS files but does not compile SCSS.

**Used by:** minima (10+ scss files), minimal-mistakes (20+ scss files). Very common in custom-themed sites. beautiful-jekyll ships pre-compiled CSS.

**What it does:** Processes `@import` directives, compiles SCSS to CSS, supports `sass.style: compressed` config.

**Complexity:** Medium-High. Requires adding a Sass compiler dependency (e.g., `grass` crate).

#### 5. Permalink Variables (`:year`, `:month`, `:day`, `:categories`)

**Problem:** Rustkyll only supports `:title` and `:collection` in permalink patterns. Jekyll supports `:year`, `:month`, `:day`, `:categories`, `:slug`, `:short_year`, `:i_month`, `:i_day`, and named permalink styles (`pretty`, `date`, `ordinal`, `none`).

**Used by:** beautiful-jekyll (`/:year-:month-:day-:title/`), minimal-mistakes (`/:categories/:title/`), choosealicense.com (`:path/`). Very common.

**What it does:** Substitutes date components and categories from post front matter into URL paths.

**Complexity:** Low-Medium. Requires extracting date parts from post filenames/front matter and substituting them.

#### 6. `absolute_url` Filter

**Problem:** Not implemented. Only `relative_url` exists.

**Used by:** beautiful-jekyll (10+ uses), minimal-mistakes (8+ uses). Very common.

**What it does:** Prepends `site.url` + `site.baseurl` to a path.

**Complexity:** Low. Simple string concatenation filter.

#### 7. `baseurl` Config Option

**Problem:** Not parsed from config. Many sites deploy to subpaths (e.g., GitHub project pages at `/repo-name/`).

**Used by:** minimal-mistakes, many GitHub project pages. Common.

**What it does:** Prepends a base path to all generated URLs. Used by `relative_url` and `absolute_url` filters.

**Complexity:** Low. Add to config, use in URL generation and filters.

#### 8. `site.categories` and `site.tags`

**Problem:** Posts have categories and tags in front matter, but rustkyll does not build `site.categories` or `site.tags` mappings, and does not generate category/tag archive pages.

**Used by:** minimal-mistakes (category and tag archive layouts), beautiful-jekyll (tags.html page). Common in blogs.

**What it does:** `site.categories` is a hash mapping category name to array of posts. `site.tags` is the same for tags. Some sites generate archive pages per category/tag.

**Complexity:** Medium. Building the maps is easy; generating archive pages requires additional logic.

#### 9. `include` with Parameters

**Problem:** Partially working. Jekyll's `{% include file.html param="value" %}` creates an `include` object accessible inside the included file. Rustkyll's current implementation does not fully support `include["param"]` access (causes "Unknown index" errors in DTC site).

**Used by:** beautiful-jekyll (`{% include header.html type="post" %}`), minimal-mistakes (extensively), DTC site (`include["max_posts"]`). Very common.

**Complexity:** Medium. Need to parse include parameters and inject them into the include's rendering context.

#### 10. Front Matter Defaults (beyond layout)

**Problem:** Rustkyll only reads `layout` from defaults. Jekyll defaults can set any front matter key (e.g., `comments: true`, `social-share: true`, `author_profile: true`, `read_time: true`).

**Used by:** beautiful-jekyll, minimal-mistakes. Common.

**Complexity:** Low-Medium. Generalize the defaults mechanism to apply arbitrary key-value pairs.

### MEDIUM PRIORITY -- Useful but less common

#### 11. `page.previous` and `page.next`

**Problem:** Not implemented. Jekyll provides `page.previous` and `page.next` for posts, allowing prev/next navigation.

**Used by:** minimal-mistakes (`post_pagination.html`), beautiful-jekyll (post layout). Fairly common in blog templates.

**Complexity:** Low. Sort posts by date and inject prev/next into each post's context.

#### 12. `site.static_files`

**Problem:** Not exposed in template context. Jekyll provides `site.static_files` as an array of objects with `path`, `modified_time`, `name`, `basename`, `extname`.

**Used by:** beautiful-jekyll (head.html). Occasional use.

**Complexity:** Medium. Need to enumerate static files and build objects with metadata.

#### 13. `number_of_words` Filter

**Problem:** Not implemented.

**Used by:** minimal-mistakes (reading time calculation), beautiful-jekyll (reading time). Fairly common.

**Complexity:** Low. Simple word count filter.

#### 14. `group_by` Filter

**Problem:** Not implemented.

**Used by:** minimal-mistakes (posts by year/category layout). Occasional.

**Complexity:** Low-Medium. Groups an array by a property, returns `{name, items}` pairs.

#### 15. `xml_escape` Filter

**Problem:** Not implemented as a custom filter. The `liquid` crate's stdlib may provide `escape` but not the XML-specific variant.

**Used by:** beautiful-jekyll (feed.xml, head.html). Common in feed generation.

**Complexity:** Low.

#### 16. Excerpt Separator Customization

**Problem:** Rustkyll uses `<!--more-->` as the excerpt separator. Jekyll's default is `"\n\n"` (first paragraph) and it can be customized via `excerpt_separator` in config.

**Used by:** minimal-mistakes (`excerpt_separator: "\n\n"`). Fairly common.

**Complexity:** Low. Read from config and use in excerpt splitting.

#### 17. Include Paths with Slashes

**Problem:** The Liquid parser rejects include filenames containing `/` (e.g., `{% include course-structured-data/mlops-zoomcamp-structured-data.html %}`). Jekyll allows subdirectory includes.

**Used by:** DTC site (4 posts fail because of this). Occasional.

**Complexity:** Medium. May require custom include tag handling or Liquid parser configuration.

#### 18. `jekyll-seo-tag` Plugin

**Problem:** Not implemented. Generates `<meta>` tags for Open Graph, Twitter Cards, canonical URLs, JSON-LD, etc.

**Used by:** choosealicense.com, minima (optional), many GitHub Pages sites. Very common.

**Complexity:** Medium-High. Substantial template logic to replicate.

#### 19. `jekyll-redirect-from` Plugin

**Problem:** Not implemented. Generates HTML redirect pages when `redirect_from` or `redirect_to` is in front matter.

**Used by:** choosealicense.com, minimal-mistakes docs. Common.

**Complexity:** Low-Medium. Generate simple HTML meta-refresh redirect pages.

#### 20. Future Posts Filtering

**Problem:** Not implemented. Jekyll's `future: false` (the default) hides posts with dates in the future.

**Used by:** Default Jekyll behavior. Potentially affects any blog.

**Complexity:** Low. Filter posts by date during collection loading.

### LOW PRIORITY -- Rare or niche features

#### 21. `_drafts/` Directory

**Problem:** Not supported. Jekyll treats `_drafts/` as unpublished posts (only shown with `--drafts` flag).

**Used by:** minimal-mistakes has a drafts directory. Occasional.

**Complexity:** Low. Add `--drafts` CLI flag, load from `_drafts/` with current date.

#### 22. CSV and JSON Data Files

**Problem:** Rustkyll only loads YAML data files. Jekyll also supports `.csv` and `.json` in `_data/`.

**Used by:** Some sites use JSON data files. Uncommon.

**Complexity:** Low. Add JSON deserialization (already have `serde_json` dependency), CSV would need a new dependency.

#### 23. CoffeeScript Compilation

**Problem:** Not supported. Deprecated feature, rarely used.

**Used by:** None of the test sites. Very rare.

**Complexity:** N/A -- not worth implementing.

#### 24. `case/when` Liquid Tag

**Problem:** Should be handled by the `liquid` crate's stdlib. Needs verification.

**Used by:** minimal-mistakes (5+ files). Fairly common.

**Complexity:** Low (verify stdlib support, add if missing).

#### 25. Timezone Handling

**Problem:** `timezone` config option is not parsed or used. All dates are treated as UTC.

**Used by:** beautiful-jekyll (`timezone: "America/Toronto"`). Occasional.

**Complexity:** Low-Medium. Parse timezone config, apply to date operations.

#### 26. Custom Ruby Plugins/Generators

**Problem:** Cannot run Ruby code. Sites that depend on custom Ruby generators will never work.

**Used by:** Complex Jekyll sites. Out of scope for rustkyll.

**Complexity:** N/A -- out of scope.

#### 27. `jekyll-github-metadata` Plugin

**Problem:** Not implemented. Injects GitHub repository info into `site.github` object.

**Used by:** choosealicense.com. GitHub Pages specific.

**Complexity:** Medium. Requires GitHub API calls or static stubs.

#### 28. `jekyll-gist` Plugin

**Problem:** Not implemented. Embeds GitHub gists.

**Used by:** minimal-mistakes. Occasional.

**Complexity:** Low. Generate `<script>` embed tags.

#### 29. Internationalization / Multi-language

**Problem:** Not supported. Very few Jekyll sites use this.

**Used by:** None of the test sites.

**Complexity:** High. Not recommended.

#### 30. `jekyll-optional-front-matter` and `jekyll-relative-links`

**Problem:** Not implemented.

**Used by:** Rare.

**Complexity:** Low-Medium each.

## Recommended Implementation Priority

Based on how commonly features appear and how severely they block building external sites, here is the recommended order for new issues:

### Phase 1 -- Unblock External Sites (Critical)

1. **Flexible config parsing** -- Make `url`/`name`/`title` optional with defaults, handle `twitter` as any value type, ignore unknown keys. This alone would unblock ALL 4 test sites from at least starting to parse.
2. **`baseurl` support** -- Add `baseurl` to config, use in `relative_url` and new `absolute_url` filter.
3. **`absolute_url` filter** -- Trivial once `baseurl` is added.
4. **Extended permalink variables** -- Support `:year`, `:month`, `:day`, `:categories`, `:slug`, and named styles (`pretty`, `date`, etc.).

### Phase 2 -- Core Blog Features (High Priority)

5. **Pagination** (`jekyll-paginate`) -- Essential for blog homepage layouts.
6. **`site.categories` and `site.tags`** -- Build mappings from post metadata, expose in template context.
7. **Include parameters** -- Fix `include["param"]` access so parameterized includes work properly.
8. **Front matter defaults generalization** -- Apply arbitrary defaults, not just `layout`.
9. **`page.previous` / `page.next`** -- Common navigation feature.

### Phase 3 -- Filters and Extras (Medium Priority)

10. **Missing filters** -- `number_of_words`, `group_by`, `xml_escape`, `sort_natural`, `concat`, `compact`, `truncatewords`.
11. **Sass/SCSS compilation** -- Add `grass` crate dependency for SCSS processing.
12. **Excerpt separator config** -- Support `excerpt_separator` from config.
13. **Include subdirectory paths** -- Fix Liquid parser to allow `/` in include names.
14. **`jekyll-seo-tag` equivalent** -- Generate meta tags.
15. **`jekyll-redirect-from`** -- Generate redirect pages.
16. **Future posts filtering** -- Respect `future: false`.

### Phase 4 -- Polish (Low Priority)

17. **`_drafts/` support** with `--drafts` flag.
18. **JSON data files** in `_data/`.
19. **`site.static_files`** API in templates.
20. **Timezone handling**.
21. **`jekyll-gist` embed**.

## Summary

Rustkyll successfully builds the DataTalks.Club site (777 pages, 8 minor warnings) because it was specifically tailored for that site's feature set. However, it cannot build ANY of the 4 external test sites tested. The primary blocker is rigid config parsing -- every site fails at config loading before even reaching template rendering.

The DataTalks.Club site does not use pagination, Sass, categories/tags, baseurl, or most Jekyll plugins, which is why rustkyll never needed these features. For rustkyll to become a practical drop-in Jekyll replacement for other sites, the priority items in Phase 1 and Phase 2 above are essential.

The good news is that many of the missing features are low-to-medium complexity. Fixing config flexibility alone would unlock the ability to at least attempt building external sites, revealing further template and rendering issues to address iteratively.
