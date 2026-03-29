# Issue 530: SEO tag generates description from page content when no explicit description set

## Problem

When a page has no `description` in front matter and `site.description` is not set in
`_config.yml`, Jekyll's `jekyll-seo-tag` does NOT emit `<meta name="description">`,
`<meta property="og:description">`, or include `description` in the JSON-LD block.

Rustkyll incorrectly falls back to generating a description from the page's rendered
content (body text). This produces nonsensical descriptions (e.g., CSS rules from the
404 page, post listing text from the index page).

### Affected pages (minima)

- **404.html**: description is CSS code: `.container { margin: 10px auto; max-width: 600px...`
- **index.html**: description is post listing: `May 20, 2016 Welcome To Jekyll May 20, 2016...`
- **about/index.html**: description is page body: `About This is the base Jekyll theme...`

All three non-post pages get spurious description/og:description meta tags and description
in the JSON-LD block.

### Example

Jekyll (correct -- no description tags when none configured):
```html
<meta property="og:type" content="website" />
<meta name="twitter:card" content="summary" />
<script type="application/ld+json">
{"@context":"https://schema.org","@type":"WebSite","url":"/"}</script>
```

Rustkyll (wrong -- fabricates description from page content):
```html
<meta name="description" content="May 20, 2016 Welcome To Jekyll..." />
<meta property="og:description" content="May 20, 2016 Welcome To Jekyll..." />
<meta property="og:type" content="website" />
<meta name="twitter:card" content="summary" />
<script type="application/ld+json">
{"@context":"https://schema.org","@type":"WebSite","description":"May 20, 2016...","url":"/"}</script>
```

## Root Cause

In `seo_tag.rs`, the description computation likely falls back to `page.content` or
`page.excerpt` when no explicit `description` is set. Jekyll's `jekyll-seo-tag` only
uses `page.excerpt` for posts (where `page.date` is set), and only when the excerpt
is meaningful. For standalone pages without a description, it emits nothing.

## Dependencies

None.

## Scope

- Fix the description fallback logic in SEO tag generation
- Only use excerpt-based description for posts (when `page.date` is present)
- Never use raw page content as a description fallback
- Ensure posts with explicit descriptions still work

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes
- [ ] DTC DOM baseline: 790/790 (must not regress)
- [ ] Minima 404.html: no `<meta name="description">` tag
- [ ] Minima 404.html: no `<meta property="og:description">` tag
- [ ] Minima 404.html: JSON-LD does NOT contain `"description"` key
- [ ] Minima index.html: no description/og:description meta tags
- [ ] Minima about/index.html: no description/og:description meta tags
- [ ] Post pages with explicit descriptions still emit description tags correctly
- [ ] At least 3 new unit tests

## Test Scenarios

### Unit: description suppression for non-post pages
- Page with no description, no excerpt, no site.description -> no description tags emitted
- Page with explicit front matter `description:` -> description tags emitted
- Post (has page.date) with excerpt -> description tags emitted from excerpt

### Unit: JSON-LD description
- Page without description -> JSON-LD omits "description" field
- Page with description -> JSON-LD includes "description" field

### Integration: minima build
- Build minima, verify 404.html has no description meta tags
- Build minima, verify index.html has no description meta tags
- Build minima, verify about/index.html has no description meta tags

## Baselines

- DTC: 790/790
- Minima: 0/9 (this fix should eliminate ~4 diffs per affected page on 3 pages = ~12 diffs)
