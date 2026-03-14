# Issue 38: Support `{% seo %}` Tag (Jekyll SEO Tag Plugin)

## Problem

Cross-site testing (Issue 32) revealed that sites using the `jekyll-seo-tag` plugin fail to build because the `{% seo %}` Liquid tag is not recognized.

The `jekyll-seo-tag` plugin is one of the most widely-used Jekyll plugins. It generates:
- `<title>` tag
- `<meta name="description">` tag
- Open Graph (`og:*`) meta tags
- Twitter Card meta tags
- JSON-LD structured data
- Canonical URL link

## Found In

- `alexeygrigorev/aihero` -- uses `{% seo %}` in its layout
- The DataTalks.Club site itself has `jekyll-seo-tag` as a dependency in `Gemfile.lock` (via the GitHub Pages gem), though it may not use `{% seo %}` directly in templates

## Scope

### In Scope

Implement a custom Liquid tag `{% seo %}` that generates SEO meta tags from page front matter and site config. The tag should be registered in `TemplateEngine::builder()` alongside the existing include tag.

The tag reads from:
- **Page front matter** (`page.title`, `page.description`, `page.image`, `page.author`)
- **Site config** (`site.title`, `site.description`, `site.url`, `site.locale`, `site.twitter`, `site.logo`, `site.author`)

Generated output (in this order):
1. `<title>` -- `page.title | site.title` or `page.title - site.title` if both exist
2. `<meta name="description">` -- from `page.description` or `page.excerpt` or `site.description`
3. `<link rel="canonical">` -- page's full URL
4. `<meta property="og:title">` -- same as title
5. `<meta property="og:description">` -- same as description
6. `<meta property="og:url">` -- canonical URL
7. `<meta property="og:site_name">` -- `site.title`
8. `<meta property="og:type">` -- "article" for posts, "website" otherwise
9. `<meta property="og:image">` -- from `page.image` if present
10. `<meta property="og:locale">` -- from `site.locale` or default "en_US"
11. `<meta name="twitter:card">` -- "summary_large_image" if image present, "summary" otherwise
12. `<meta name="twitter:site">` -- from `site.twitter.username` if present
13. JSON-LD `<script type="application/ld+json">` -- basic WebPage/BlogPosting schema

The tag should also support `{% seo title=false %}` which suppresses the `<title>` tag (some sites set title separately).

### Out of Scope

- Full parity with every edge case in the Ruby `jekyll-seo-tag` gem
- Facebook app ID / Facebook admin meta tags
- Google site verification meta tags
- Complex author lookup (looking up authors from `site.data`)

## Dependencies

- None (the custom tag infrastructure already exists via `LenientIncludeTag` pattern)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] A `{% seo %}` tag is implemented in `src/template/seo_tag.rs` (or similar)
- [ ] The tag is registered in all `TemplateEngine` constructors (`new()`, `with_includes()`, `with_includes_map()`)
- [ ] Given a page with `title: "My Page"` and site with `title: "My Site"`, `{% seo %}` generates `<title>My Page - My Site</title>`
- [ ] Given a page with `title: "My Page"` and site with `title: "My Site"` and `url: "https://example.com"`, the output includes `<meta property="og:title">`
- [ ] Given a page with `description: "A description"`, the output includes `<meta name="description" content="A description">`
- [ ] Given no page description but `site.description` set, it falls back to `site.description`
- [ ] `{% seo title=false %}` omits the `<title>` tag but still generates meta tags
- [ ] Given `page.image: "/img/cover.png"` and `site.url: "https://example.com"`, the output includes `<meta property="og:image" content="https://example.com/img/cover.png">`
- [ ] The generated HTML is well-formed (all tags properly closed/self-closed)
- [ ] `cargo test` passes with all new and existing tests

### Output Verification

- [ ] Build a test site that uses `{% seo %}` in a layout and verify the generated HTML contains the expected meta tags
- [ ] The generated meta tags should be inside the `<head>` section (the tag itself just outputs HTML; placement depends on the layout)
- [ ] Verify that HTML entities in title/description are properly escaped in meta tag attributes

## Test Scenarios

### Unit: Title generation

- Page title + site title produces `<title>Page Title - Site Title</title>`
- Page title only (no site title) produces `<title>Page Title</title>`
- Site title only (no page title) produces `<title>Site Title</title>`
- Neither title set produces no `<title>` tag
- `{% seo title=false %}` suppresses `<title>` tag

### Unit: Description meta tag

- Page description present: generates `<meta name="description" content="...">`
- No page description, site description present: falls back to site description
- No description anywhere: omits the description meta tag
- Description with HTML entities (quotes, ampersands) is properly escaped

### Unit: Open Graph tags

- `og:title` matches the page/site title
- `og:description` matches the description
- `og:url` is the full canonical URL (site.url + page.url)
- `og:site_name` is the site title
- `og:type` is "article" when page has a `date` field, "website" otherwise
- `og:image` is generated when `page.image` is set, uses absolute URL
- `og:image` is omitted when no image is set
- `og:locale` uses `site.locale` or defaults to "en_US"

### Unit: Twitter Card tags

- Card type is "summary_large_image" when an image is present
- Card type is "summary" when no image
- `twitter:site` is generated from `site.twitter.username` (with `@` prefix)
- `twitter:site` is omitted when no twitter config

### Unit: JSON-LD structured data

- Generates valid JSON-LD wrapped in `<script type="application/ld+json">`
- Uses "BlogPosting" type for posts (pages with `date`), "WebPage" otherwise
- Includes `name`, `headline`, `description`, `url` fields

### Unit: HTML escaping

- Title containing `&`, `<`, `>`, `"` is properly escaped in all meta tags
- Description containing quotes is properly escaped in `content` attributes

### Integration: Full tag rendering

- Render `{% seo %}` with a complete page context (title, description, image, date) and verify all expected meta tags are present
- Render `{% seo %}` with minimal context (just site.title) and verify graceful output with no errors
- Render `{% seo %}` with no context at all and verify it produces empty or minimal output without errors

## Implementation Notes

- Follow the `LenientIncludeTag` pattern in `src/template/include_tag.rs` for implementing a custom Liquid tag
- The tag needs access to the template runtime to read `page.*` and `site.*` variables
- Implement as `TagReflection + ParseTag` traits
- The rendered tag should write its output as a single block of HTML
- HTML-escape all user-provided values before inserting into attribute values
- The tag must be registered in `TemplateEngine::builder()` using `.tag(SeoTag)` in all three constructors that build parsers
