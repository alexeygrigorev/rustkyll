# Issue 53: Fix minimal-mistakes site build

## Problem

minimal-mistakes (1 page) builds with Jekyll in 0.9s but fails with rustkyll.
The build fails with `Unknown tag 'include_cached'` because the theme uses the
`jekyll-include-cache` plugin, which provides the `include_cached` Liquid tag.

## Root Cause

The `jekyll-include-cache` plugin adds an `include_cached` tag that is
functionally identical to `include` but caches the rendered output so it is only
evaluated once per unique set of parameters. Rustkyll does not recognize this
tag.

## Goal

Support the `include_cached` tag so minimal-mistakes builds and produces correct
output matching Jekyll.

## Approach

1. Register `include_cached` as an alias for the existing `LenientIncludeTag`
   in the template engine. Since rustkyll does not do incremental re-rendering,
   there is no caching benefit to implement -- the tag should behave identically
   to `include`.
2. Update `preprocess_include_paths` to also handle `include_cached` tags
   (subdirectory path quoting, escaped quotes, dynamic includes).
3. Build minimal-mistakes and verify output.

## Dependencies

- Issue 51 (benchmark-page-count-accuracy) -- must be `.done.md` (already done;
  fixes the YAML null-value config parsing that previously blocked this site).

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (all existing tests plus new tests)
- [ ] `include_cached` tag is registered in the template engine and works
      identically to `include` (same parameter passing, same partial lookup)
- [ ] `preprocess_include_paths` handles `include_cached` tags the same way it
      handles `include` tags (path quoting, escaped quotes, dynamic paths)
- [ ] `cargo run --release -- build --source websites/minimal-mistakes`
      completes without errors
- [ ] Output page count matches Jekyll (1 HTML page: `index.html`)
- [ ] The generated `index.html` contains the expected structural elements from
      the default layout: `<!doctype html>`, `<html`, masthead markup,
      footer markup
- [ ] The included partials (`skip-links.html`, `masthead.html`, `footer.html`)
      are rendered in the output (not missing or empty)
- [ ] No regressions: all previously-passing sites still build correctly
- [ ] No raw Liquid tags (`{%`, `{{`) appear in the generated HTML output

## Test Scenarios

### Unit: preprocess_include_paths handles include_cached

- `{% include_cached foo.html %}` is left unchanged (no path quoting needed)
- `{% include_cached subdir/foo.html %}` gets the path quoted to
  `{% include_cached "subdir/foo.html" %}`
- `{% include_cached foo.html param="val" %}` is left unchanged
- `{%- include_cached foo.html -%}` preserves whitespace-control markers
- Mixed template with both `include` and `include_cached` tags: both are
  processed correctly
- `{% include_cached {{ page.partial }} %}` dynamic include is handled
  (sentinel rewrite)

### Unit: include_cached tag registration

- Template engine with includes recognizes `include_cached` as a valid tag
- `{% include_cached foo.html locale=locale %}` renders the same output as
  `{% include foo.html locale=locale %}`
- `{% include_cached foo.html %}` with a missing partial produces the same error
  as `{% include foo.html %}` would

### Integration: minimal-mistakes site build

- Build minimal-mistakes with rustkyll, verify exit code 0
- Verify `_site/index.html` exists and is non-empty
- Verify the output HTML contains `<!doctype html>` and `</html>`
- Verify the output contains rendered content from `include_cached` partials
  (masthead, footer, skip-links)
- Verify no raw `{% include_cached` tags appear in the output

## Output Quality Verification

After fixing the build, structurally compare rustkyll output against Jekyll output:

1. Same HTML files generated (file tree diff)
2. For each HTML file, compare structural elements: title, headings (h1-h6), links, images
3. No missing pages, no empty pages, no raw Liquid tags in output

### Visual comparison with Playwright

Sites MUST be served over HTTP so CSS, images, fonts, and JS all load. Serve
Jekyll _site/ and rustkyll _site/ on separate ports (e.g. python -m http.server).
Use Playwright to screenshot key pages from both fully-rendered sites and compare.
Verify no 404s in browser console. Flag any visual differences beyond a minor
threshold.
