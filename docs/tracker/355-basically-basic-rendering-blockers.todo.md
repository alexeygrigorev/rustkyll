# Issue 355: Basically Basic rendering blockers

## Problem

DOM comparison of the Basically Basic theme demo site (issue #242) found 0/18 pages matching. Several rendering issues prevent accurate output. This issue tracks the Basically Basic-specific blockers.

## Blockers Found (from #242)

### 1. Author/image hash serialization in SEO tags
`site.author` (a YAML hash with `name`, `twitter`, `picture`) renders as a flat string `__key_ordernametwitterpicture...` instead of being accessible as a Liquid hash in `jekyll-seo-tag` includes. Similarly, `page.image` (hash with `path`, `thumbnail`, `caption`) is flattened. Affects all 18 common pages.

### 2. baseurl not prepended to SEO/meta URLs
Canonical URLs, `og:url`, and JSON-LD `url` fields are missing the `/example` baseurl prefix. Jekyll prepends `site.baseurl` to these URLs automatically. Affects all 18 common pages.

### 3. Future post filtering
Jekyll excludes posts dated in the future (e.g., `9999-12-31`) by default unless `future: true` is set. Rustkyll includes them, producing an extra page.

### 4. Liquid-in-SCSS processing
The theme's `main.scss` contains `{{ site.data.theme.skin | default: 'default' }}` which Jekyll processes before SCSS compilation. Rustkyll fails to compile this SCSS. Related to #249 (Mediumish) and #345 (al-folio).

### 5. site.tags / site.categories iteration
Tag and category archive pages use `{% for tag in site.tags %}` with `tag[1].size` to iterate. These render as empty pages in rustkyll, producing 0 bytes for `tags/index.html` and `categories/index.html`.

### 6. Category case in permalinks
Jekyll lowercases categories in permalink URLs; rustkyll preserves original case. Related to #354 (Hydeout).

### 7. Syntax highlighting class differences
Rouge-style syntax highlighting `<span>` classes differ from rustkyll's output (282 differences in `markup-syntax-highlighting.html`).

### 8. og:locale format
Jekyll converts `lang: en-US` to `en_US` for `og:locale` meta tag; rustkyll outputs `en-US` as-is.

## Acceptance Criteria

- [ ] Each blocker is either fixed or tracked in a dedicated cross-theme issue.

## Dependencies

- #242 (benchmark baseline)
- Related: #249, #345 (SASS import), #354 (category URL case)
