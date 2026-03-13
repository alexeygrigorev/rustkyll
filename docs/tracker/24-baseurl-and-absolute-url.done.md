# Issue 24: baseurl Support and absolute_url Filter

## Problem

Many Jekyll sites deploy to subpaths (e.g., GitHub project pages at `/repo-name/`). Rustkyll does not parse `baseurl` from config and does not have an `absolute_url` filter. The existing `relative_url` filter already reads `site.baseurl` from the runtime context, but the config parser does not extract `baseurl` and `build_site_context` does not populate it.

## Requirements

- Parse `baseurl` from `_config.yml` as a named field with default `""` (empty string)
- Ensure `build_site_context` populates `site.baseurl` in the template context
- Implement `absolute_url` filter: prepends `site.url` + `site.baseurl` to a path
- Register the `absolute_url` filter in the template engine
- Verify existing `relative_url` filter works correctly with `baseurl` now being populated
- All existing tests must continue to pass

## Scope

- `src/config.rs` -- add `baseurl` field
- `src/generator.rs` -- populate `site.baseurl` in `build_site_context`
- `src/template/filters/` -- new `absolute_url.rs` filter module
- `src/template/filters/mod.rs` -- register the new filter
- `src/template/engine.rs` -- register the new filter with the Liquid parser

## Dependencies

- Issue #23 (flexible config parsing) should be done first. If `baseurl` is already captured by the catch-all extras map from issue #23, this issue still needs to promote it to a named field so it has a proper default and is reliably accessible. However, this issue CAN be implemented independently if #23 is not yet done -- just add the field to the current `SiteConfig`.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `SiteConfig` has a `baseurl` field that defaults to `""` when not present in YAML
- [ ] `build_site_context` inserts `site.baseurl` into the Liquid context
- [ ] `{{ page.url | absolute_url }}` renders correctly in templates
- [ ] `absolute_url` filter prepends `site.url + site.baseurl` to the input path
- [ ] `absolute_url` handles edge cases: trailing slashes on url/baseurl, leading slashes on input path, empty inputs
- [ ] `relative_url` continues to work correctly (it already reads `site.baseurl` from context)
- [ ] A config with `baseurl: /blog` results in `relative_url` prepending `/blog` and `absolute_url` prepending `https://example.com/blog`
- [ ] A config with no `baseurl` results in `relative_url` and `absolute_url` working as before (no subpath)
- [ ] The DTC site still builds correctly (it has no `baseurl`, so default empty string applies)

## Test Scenarios

### Unit: Config parsing of baseurl

- Parse YAML with `baseurl: "/blog"` -- verify `config.baseurl` is `"/blog"`
- Parse YAML with `baseurl: ""` -- verify `config.baseurl` is `""`
- Parse YAML with no `baseurl` key -- verify `config.baseurl` defaults to `""`
- Parse YAML with `baseurl: /repo-name` (no quotes) -- verify it parses as `"/repo-name"`

### Unit: absolute_url filter (isolated, no runtime context)

These tests use `call_filter!` without runtime context, so `site.url` and `site.baseurl` will not be available. The filter should gracefully handle missing context.

- Input `"/about.html"` with no site context -- verify output is `"/about.html"` (or just the path itself, since no url/baseurl available)

### Unit: absolute_url filter (with runtime context)

These tests need a full Liquid render with a context containing `site.url` and `site.baseurl`.

- `site.url = "https://example.com"`, `site.baseurl = ""`, input `"/about.html"` -- output `"https://example.com/about.html"`
- `site.url = "https://example.com"`, `site.baseurl = "/blog"`, input `"/about.html"` -- output `"https://example.com/blog/about.html"`
- `site.url = "https://example.com"`, `site.baseurl = "/blog"`, input `"about.html"` (no leading slash) -- output `"https://example.com/blog/about.html"`
- `site.url = "https://example.com/"` (trailing slash), `site.baseurl = "/blog"`, input `"/page"` -- output `"https://example.com/blog/page"` (no double slashes)
- `site.url = "https://example.com"`, `site.baseurl = "/blog/"` (trailing slash), input `"/page"` -- output `"https://example.com/blog/page"` (no double slashes)
- `site.url = ""`, `site.baseurl = ""`, input `"/about.html"` -- output `"/about.html"`
- `site.url = ""`, `site.baseurl = "/blog"`, input `"/about.html"` -- output `"/blog/about.html"`
- Empty input `""` with `site.url = "https://example.com"`, `site.baseurl = ""` -- output `"https://example.com"` or `"https://example.com/"`

### Unit: relative_url filter with baseurl populated

- Render `{{ "/assets/style.css" | relative_url }}` with `site.baseurl = "/blog"` in context -- verify output is `"/blog/assets/style.css"`
- Render `{{ "/assets/style.css" | relative_url }}` with `site.baseurl = ""` in context -- verify output is `"/assets/style.css"`

### Integration: Site context includes baseurl

- Build site context from config with `baseurl: "/blog"` -- verify `site.baseurl` is present and equals `"/blog"`
- Build site context from config with no baseurl -- verify `site.baseurl` is present and equals `""`
- Render a template `{{ "/page" | absolute_url }}` through the full template engine with a site context built from config `url: "https://example.com"`, `baseurl: "/blog"` -- verify output `"https://example.com/blog/page"`

### Integration: Template rendering end-to-end

- Create a minimal page with template content `<a href="{{ "/about" | absolute_url }}">About</a>` -- verify the generated HTML contains `href="https://example.com/blog/about"` when config has `url: "https://example.com"` and `baseurl: "/blog"`
- Create a page with `{{ "/style.css" | relative_url }}` -- verify output is `"/blog/style.css"` with `baseurl: "/blog"`

### Regression: DTC site

- Verify the DTC site config (which has no `baseurl`) still builds and `relative_url` filter output is unchanged

## Notes

- The `absolute_url` filter implementation should follow the same pattern as `relative_url`: read `site.url` and `site.baseurl` from the Liquid runtime context using `runtime.try_get()`.
- Jekyll's `absolute_url` behavior: strips trailing slash from url, strips trailing slash from baseurl, ensures exactly one slash between url+baseurl and between baseurl+path. It does NOT add a trailing slash to the result.
- The filter must be registered in two places: (1) the filter module (`mod.rs`) and (2) the template engine builder (wherever `RelativeUrl` is registered with the Liquid `ParserBuilder`).

## References

- Issue #22 compatibility research, gaps #6 and #7
- `src/template/filters/relative_url.rs` -- existing filter that already reads `site.baseurl`
- `src/config.rs` -- `SiteConfig` struct
- `src/generator.rs` -- `build_site_context` function
- `src/template/engine.rs` -- where filters are registered with the Liquid parser
