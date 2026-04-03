# Issue 515: SEO tag JSON-LD uses wrong @type, name, and missing sameAs for non-post pages

## Problem

The JSON-LD output has three issues visible on chirpy's about page and index page:

### 1. `@type` is `BlogPosting` instead of `WebSite` for non-post pages

The about page (a tab/page, not a post) gets `@type: BlogPosting` because rustkyll
treats any document with a date as an article. Jekyll's SEO tag uses `BlogPosting`
only for posts (documents in the `_posts` collection). For standalone pages and
non-post collection items, it uses `WebSite` (or `WebPage`).

**Jekyll:**
```json
{"@type": "WebSite", "name": "your_full_name", ...}
```

**Rustkyll:**
```json
{"@type": "BlogPosting", "name": "Chirpy", "mainEntityOfPage": {"@type": "WebPage", "@id": "/about/"}, ...}
```

### 2. `name` should be `site.social.name` (or `site.author.name`), not `site.title`

Jekyll's SEO tag uses `site.social.name` as the JSON-LD `name` field when available,
falling back to the first author name or site title. Rustkyll always uses `site.title`.

Chirpy config:
```yaml
social:
  name: your_full_name
```

### 3. `sameAs` missing

Jekyll's SEO tag emits `sameAs` from `site.social.links` array. Rustkyll does not
read this config key at all.

Chirpy config:
```yaml
social:
  links:
    - https://twitter.com/username
    - https://github.com/username
```

### 4. Extra `mainEntityOfPage` for non-post pages

When `@type` is `WebSite`, Jekyll does not emit `mainEntityOfPage`. Rustkyll emits
it unconditionally for any page with a date.

## Affected Pages

- chirpy: `about/index.html` (4 diffs: @type, name, sameAs, mainEntityOfPage)
- chirpy: `index.html` (2 diffs: name, sameAs)
- Any site using `site.social.name` and `site.social.links` in config

## Fix

In `src/template/seo_tag.rs`, in the JSON-LD generation section:

1. Check `page.collection` or `page.is_post` -- only use `BlogPosting` for posts
2. Read `site.social.name` and use it for JSON-LD `name` when available
3. Read `site.social.links` and emit as `sameAs` array
4. Only emit `mainEntityOfPage` for `BlogPosting` type, not `WebSite`

## Dependencies

None.

## Baseline

- DTC: 790/790 (must not regress)
- Chirpy: 12/17 (should not regress; about page may improve)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] Non-post pages (pages, tabs) get `@type: WebSite` in JSON-LD, not `BlogPosting`
- [ ] Post pages still get `@type: BlogPosting`
- [ ] JSON-LD `name` uses `site.social.name` when available, falling back to author name then site title
- [ ] JSON-LD `sameAs` is emitted from `site.social.links` array when present
- [ ] `mainEntityOfPage` only emitted for `BlogPosting`, not `WebSite`
- [ ] DTC DOM baseline remains at 790/790
- [ ] Chirpy DOM match count does not drop below 12/17

## Test Scenarios

### Unit: JSON-LD @type selection
- Page with `collection: "posts"` -- verify @type = BlogPosting
- Page without collection (standalone page) -- verify @type = WebSite
- Page with collection = "tabs" (non-post) -- verify @type = WebSite

### Unit: JSON-LD name field
- site.social.name = "John Doe" -- verify name = "John Doe"
- site.social.name absent, site.author = "Jane" -- verify name = "Jane"
- Both absent, site.title = "My Blog" -- verify name = "My Blog"

### Unit: JSON-LD sameAs
- site.social.links = ["https://twitter.com/user", "https://github.com/user"] -- verify sameAs array
- site.social.links absent -- verify no sameAs field

### Unit: mainEntityOfPage
- BlogPosting type -- verify mainEntityOfPage present
- WebSite type -- verify mainEntityOfPage absent
