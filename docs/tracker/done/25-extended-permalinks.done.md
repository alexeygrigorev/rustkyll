# Issue 25: Extended Permalink Variables

## Problem

Rustkyll only supports `:title` and `:collection` in permalink patterns (see `generate_url()` in `src/collection.rs`). Jekyll supports `:year`, `:month`, `:day`, `:categories`, `:slug`, `:short_year`, `:i_month`, `:i_day`, and named styles (`pretty`, `date`, `ordinal`, `none`).

This prevents external Jekyll sites from building correctly. For example, beautiful-jekyll uses `/:year-:month-:day-:title/`, minimal-mistakes uses `/:categories/:title/`, and choosealicense.com uses `:path/`.

## Scope

Extend `generate_url()` and its callers to support the full set of Jekyll permalink variables and named styles.

### In scope

- Support `:year`, `:month`, `:day` extracted from post filename date or front matter `date`
- Support `:short_year` (2-digit year), `:i_month` (month without leading zero), `:i_day` (day without leading zero)
- Support `:categories` (categories from front matter, joined with `/`, or empty if none)
- Support `:slug` (alias for `:title` -- in Jekyll, slug is the URL-safe version of the title from the filename)
- Support `:path` (relative path to the source file without extension)
- Support named permalink styles that expand to patterns:
  - `date` = `/:categories/:year/:month/:day/:title.html`
  - `pretty` = `/:categories/:year/:month/:day/:title/`
  - `ordinal` = `/:categories/:year/:y_day/:title.html`
  - `none` = `/:categories/:title.html`
- Per-collection permalink patterns from config (already supported via `CollectionConfig.permalink`)
- All existing tests must continue to pass

### Out of scope

- `:y_day` (day of year, 1-366) -- only used by the `ordinal` style which is very rare
- Custom slugify modes (Jekyll's `slugify` filter modes: `none`, `raw`, `default`, `pretty`, `ascii`, `latin`)
- Timezone-aware date extraction

## Dependencies

- Issue #23 (flexible config parsing) -- DONE
- Issue #24 (baseurl and absolute_url) -- DONE

## Implementation Notes

- `generate_url()` currently takes `pattern`, `collection`, and `title`. It needs to accept additional context: date components and categories.
- The `CollectionItem` already has a `date: Option<String>` field parsed from `YYYY-MM-DD-slug.md` filenames.
- Posts can also have a `date` field in front matter which should override the filename date.
- Categories come from front matter `categories` (array) or `category` (single string).
- Named styles should be expanded to their pattern before variable substitution. The expansion should happen at the config level or at the start of `generate_url()`.
- When `:categories` is empty, double slashes (`//`) should be collapsed to single slashes.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests plus new tests
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `generate_url()` (or equivalent) accepts date/category context and substitutes `:year`, `:month`, `:day`, `:short_year`, `:i_month`, `:i_day`
- [ ] `:categories` is substituted with categories joined by `/` and double slashes are collapsed when categories are empty
- [ ] `:slug` works as an alias for `:title`
- [ ] `:path` substitutes the relative source path without extension
- [ ] Named permalink styles (`date`, `pretty`, `none`) are recognized and expanded to their full pattern
- [ ] Posts with `date` in front matter use that date for permalink variables (overriding filename date)
- [ ] Posts with no date produce a sensible fallback (empty string for date variables, or skip date substitution)
- [ ] Permalink pattern `/:year-:month-:day-:title/` (beautiful-jekyll style) produces URLs like `/2021-01-15-my-post/`
- [ ] Permalink pattern `/:categories/:title/` with no categories produces `/:title/` (not `//:title/`)
- [ ] The DTC site still builds correctly with its existing `/blog/:title.html` pattern

## Test Scenarios

### Unit: Named style expansion

- `"date"` expands to `"/:categories/:year/:month/:day/:title.html"`
- `"pretty"` expands to `"/:categories/:year/:month/:day/:title/"`
- `"none"` expands to `"/:categories/:title.html"`
- A custom pattern like `"/blog/:title.html"` is left unchanged

### Unit: Date variable substitution

- Post filename `2021-03-15-my-post.md`: `:year` = `2021`, `:month` = `03`, `:day` = `15`
- `:short_year` = `21`, `:i_month` = `3`, `:i_day` = `15` (no leading zero)
- Front matter `date: 2022-06-01` overrides filename date: `:year` = `2022`, `:month` = `06`, `:day` = `01`
- Post with no date: date variables resolve to empty string or are removed cleanly

### Unit: Category substitution

- Front matter `categories: [machine-learning, tutorials]`: `:categories` = `machine-learning/tutorials`
- Front matter `category: blog`: `:categories` = `blog`
- No categories in front matter: `:categories` = empty, double slashes collapsed
- Pattern `/:categories/:year/:month/:day/:title.html` with categories `[tech]` and date `2021-03-15` and slug `hello` produces `/tech/2021/03/15/hello.html`

### Unit: Slug and path substitution

- `:slug` behaves identically to `:title` for a post with slug `my-post`
- `:path` for a file at `_posts/2021-03-15-my-post.md` produces something like `2021-03-15-my-post`

### Unit: Double-slash collapsing

- Pattern `/:categories/:title.html` with no categories produces `/:title.html` (not `//:title.html`)
- Pattern `/:categories/:year/:month/:day/:title/` with no categories produces `/:year/:month/:day/:title/`

### Integration: Existing DTC site

- DTC site post with permalink `/blog/:title.html` still generates correct URLs (e.g., `/blog/segmentation.html`)
- All existing collection URLs remain unchanged

### Integration: External site patterns

- Pattern `/:year-:month-:day-:title/` with date `2021-01-15` and slug `my-post` produces `/2021-01-15-my-post/`
- Pattern `/:categories/:title/` with categories `[updates]` and slug `hello` produces `/updates/hello/`

## References

- Issue #22 compatibility research, gap #5
- Jekyll permalink documentation: https://jekyllrb.com/docs/permalinks/
