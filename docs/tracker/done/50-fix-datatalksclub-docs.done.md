# Issue 50: Fix DataTalksClub/docs site build

## Problem

DataTalksClub/docs (57 pages, just-the-docs theme) builds with Jekyll in 1.8s but fails with rustkyll. The error is a Liquid parse failure on escaped quotes inside include tag parameters.

The failing line is in `_layouts/default.html`:

```liquid
{% include vendor/anchor_headings.html html=content beforeHeading="true" anchorBody="<svg viewBox=\"0 0 16 16\" aria-hidden=\"true\"><use xlink:href=\"#svg-link\"></use></svg>" anchorClass="anchor-heading" anchorAttrs="aria-labelledby=\"%html_id%\"" %}
```

Jekyll's include tag parser uses a regex that explicitly supports backslash-escaped double quotes inside double-quoted parameter values: `[^"\\]*(?:\\.[^"\\]*)*)`. After parsing, Jekyll unescapes them with `d_quoted.gsub('\\"', '"')`.

Rustkyll's `preprocess_include_paths` function currently handles subdirectory paths and dynamic includes, but does not handle escaped quotes in parameter values. The Liquid parser's pest grammar sees `\"` as the end of the string followed by unexpected tokens, causing a parse error.

## Root Cause

The `preprocess_include_paths` function in `src/template/include_tag.rs` must be extended to replace `\"` (backslash-escaped double quotes) inside include tag parameter values with an alternative representation before the Liquid parser sees them. After rendering, the escaped quotes should appear as literal `"` in the output.

## Approach

1. In `preprocess_include_paths`, detect include tags whose parameter values contain `\"` (backslash-escaped quotes)
2. Replace the escaped quotes with a representation the Liquid parser can handle. Two viable strategies:
   - Replace `\"` with `&quot;` (HTML entity) inside double-quoted include parameter values, since these values typically end up in HTML output anyway
   - Or replace with a unique sentinel string that gets post-processed back to `"` after rendering
3. Ensure `preprocess_include_paths` correctly identifies which `\"` sequences are inside double-quoted parameter values (not in the path or outside quotes)
4. Verify the site builds and produces correct HTML output

## Dependencies

None -- the include tag infrastructure is already in place (issues 26, 39, 41 are done).

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (all existing tests, plus new tests for escaped-quote handling)
- [ ] `cargo run --release -- build --source websites/DataTalksClub/docs` completes without errors
- [ ] The generated output contains the expected number of HTML pages (close to 57)
- [ ] The generated HTML for pages using the `default` layout contains the anchor heading SVG markup with proper double quotes (not escaped, not mangled)
- [ ] Include parameters with escaped quotes are correctly unescaped in the rendered output: `viewBox="0 0 16 16"` not `viewBox=\"0 0 16 16\"` or `viewBox=&quot;0 0 16 16&quot;`
- [ ] No regressions on currently-passing sites (datatalksclub.github.io, and any other sites tested in issue 48)
- [ ] All existing tests still pass

## Test Scenarios

### Unit: Escaped quote preprocessing

- `preprocess_include_paths` with `{% include file.html param="value with \"escaped\" quotes" %}` produces output the Liquid parser can handle
- `preprocess_include_paths` with the exact failing line from `default.html` (anchor_headings with SVG markup containing `\"`) produces parseable output
- `preprocess_include_paths` with mixed escaped and non-escaped parameters preserves non-escaped values unchanged
- `preprocess_include_paths` with single-quoted parameters containing escaped single quotes (if applicable) is handled or at least does not crash
- `preprocess_include_paths` with no escaped quotes still works identically to before (regression check)

### Unit: Escaped quote unescaping in output

- An include parameter value `"<svg viewBox=\"0 0 16 16\">"` renders as `<svg viewBox="0 0 16 16">` in the final HTML (literal double quotes, no escaping artifacts)
- An include parameter value `"aria-labelledby=\"%html_id%\""` renders with literal `"` around `%html_id%`

### Integration: DataTalksClub/docs site build

- Build `websites/DataTalksClub/docs` with `cargo run --release -- build --source websites/DataTalksClub/docs`
- Verify the build completes without errors
- Count generated HTML files; expect close to 57
- Inspect a generated page that uses the `default` layout and verify the anchor heading markup is present and well-formed
- Verify no raw Liquid tags (`{%`, `{{`, `}}`, `%}`) appear in the generated HTML

### Regression: Existing sites

- Rebuild at least one currently-passing site (e.g., datatalksclub.github.io) and verify it still produces correct output
- Run `cargo test` and verify all existing tests pass

## Output Quality Verification

After fixing the build, structurally compare rustkyll output against Jekyll output:

1. Same HTML files generated (file tree diff)
2. For each HTML file, compare structural elements: title, headings (h1-h6), links, images
3. No missing pages, no empty pages, no raw Liquid tags in output
4. RSS/Atom feeds and sitemap (if any) must match

### Visual comparison with Playwright

Sites MUST be served over HTTP so CSS, images, fonts, and JS all load. Serve Jekyll `_site/` and rustkyll `_site/` on separate ports (e.g. `python -m http.server`). Use Playwright to screenshot key pages from both fully-rendered sites and compare. Verify no 404s in browser console. Flag any visual differences beyond a minor threshold.

## Notes

- The `just-the-docs` theme is a gem-based theme. The site's `_includes/` directory contains theme overrides and the `vendor/anchor_headings.html` include that triggers this bug.
- Jekyll's include param parser regex: `([\w-]+)\s*=\s*(?:"([^"\\]*(?:\\.[^"\\]*)*)"|'([^'\\]*(?:\\.[^'\\]*)*)'|([\w.-]+))` -- this is the canonical reference for what escape sequences must be supported.
- The config has `heading_anchors: false`, which means the `{% if site.heading_anchors != false %}` branch in `default.html` should actually be skipped. However, the Liquid parser still needs to parse the entire template (including both branches of the if/else), so the escaped quotes must be handled regardless of runtime evaluation.
