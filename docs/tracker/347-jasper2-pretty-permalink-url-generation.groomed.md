# Issue 347: Jasper2 pretty permalink and canonical URL generation

## Problem

Jasper2 uses `permalink: /:title` in `_config.yml` (no `.html` extension). Jekyll treats this as a "pretty" permalink, generating output at `/<title>/index.html` so that the URL `/<title>/` works without a file extension. In the rustkyll build, many Jasper2 links and canonical URLs still point at `.html` paths, creating repeated DOM diffs across the homepage, posts, and metadata.

## Root Cause

The Jasper2 permalink pattern `/:title` does not end with `.html`. Jekyll interprets this as producing a directory with an `index.html` inside (i.e., `/<title>/index.html`), and the URL exposed to templates is `/<title>/` (with trailing slash). rustkyll's permalink expansion may be appending `.html` instead of creating the directory structure, or the URL exposed to Liquid templates may include `.html` when it should end with `/`.

## Scope

1. Verify how rustkyll handles permalink patterns without `.html` extension (e.g., `/:title`, `/:categories/:title`).
2. Fix the URL generation so that `permalink: /:title` produces:
   - Output file: `<output_dir>/<title>/index.html`
   - URL in Liquid context (`page.url`, `post.url`): `/<title>/`
3. Fix canonical URLs in `<link rel="canonical">` and `<meta property="og:url">` tags to use the pretty URL (no `.html`).
4. Verify links on the homepage, post pages, and navigation all use the pretty URL format.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with tests covering permalink patterns without `.html`
- [ ] For `permalink: /:title`, posts are output to `<output_dir>/<title>/index.html` (directory-based)
- [ ] `page.url` / `post.url` in Liquid templates resolves to `/<title>/` (trailing slash, no `.html`)
- [ ] Canonical URLs in SEO tags use the pretty URL format
- [ ] The Jasper2 homepage post links use `/<title>/` format, not `/<title>.html`
- [ ] Navigation links in Jasper2 templates use pretty URLs
- [ ] The fix is generic -- any site with `permalink: /:title` or `permalink: /:categories/:title` benefits, not just Jasper2
- [ ] The Jasper2 DOM comparison improves from the #240 baseline (fewer URL-related diffs)
- [ ] DTC DOM count remains at 788/790 or above

## Test Scenarios

### Unit: permalink expansion without .html
- `permalink: /:title` with title `"my-post"` produces URL `/my-post/` and output path `<dir>/my-post/index.html`
- `permalink: /:categories/:title` with category `"tech"` and title `"intro"` produces URL `/tech/intro/` and output path `<dir>/tech/intro/index.html`
- `permalink: /:title.html` continues to produce URL `/<title>.html` and output path `<dir>/<title>.html` (no regression)

### Unit: URL trailing slash
- Verify `page.url` ends with `/` for pretty permalinks
- Verify `page.url` ends with `.html` for explicit `.html` permalinks
- Verify `url_to_output_path` handles trailing-slash URLs correctly (already has a test)

### Integration: Jasper2 post URLs
- Build `websites/jasper2/` with rustkyll and verify posts are written to `<title>/index.html` directories
- Inspect the homepage HTML for post links -- all should be `href="/<baseurl>/<title>/"` format
- Inspect a post page's canonical URL -- should be `<production_url>/<title>/`
- Verify the Atom feed entry links use pretty URLs

### Regression: DTC site
- Build DTC site and verify its `permalink: /blog/:title.html` pattern still produces `.html` URLs and flat file output
- Verify DTC DOM count remains at 788/790 or above

## Dependencies

- Issue #240 (must be `.done.md` or `.in-progress.md`)
