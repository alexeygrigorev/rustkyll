# Issue 522: jekyll-docs @mention plugin converts @username to GitHub links

## Problem

On 3+ jekyll-docs pages, `@username` patterns (e.g. `@jekyllbot`) appear as
literal text instead of being converted to GitHub profile links. Jekyll uses the
`jekyll-mentions` plugin to convert `@username` into
`<a href="https://github.com/username" class="user-mention">@username</a>`.

### Affected pages (3+)

- docs/maintaining/special-labels/index.html (multiple @jekyllbot mentions)
- docs/maintaining/triaging-an-issue/index.html (@jekyllbot mentions)
- docs/maintaining/affinity-team-captain/index.html (@jekyllbot mentions)

### Example

Expected (Jekyll with jekyll-mentions):
```html
<a href="https://github.com/jekyllbot" class="user-mention">@jekyllbot</a>
```

Actual (rustkyll):
```html
@jekyllbot
```

## Root Cause

Rustkyll does not implement the `jekyll-mentions` plugin. This plugin scans
rendered HTML for `@username` patterns and replaces them with GitHub links.

## Scope

Implement basic jekyll-mentions-compatible @mention replacement:
1. Detect when `jekyll-mentions` is in the site's `plugins` list
2. After markdown rendering, scan text nodes for `@username` patterns
3. Replace with `<a href="https://github.com/username" class="user-mention">@username</a>`
4. Do NOT replace inside `<code>`, `<pre>`, or `<a>` tags

The base URL defaults to `https://github.com` but can be configured via
`jekyll-mentions.base_url` in `_config.yml`.

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] @mentions are only processed when jekyll-mentions is in plugins list
- [ ] `@jekyllbot` renders as `<a href="https://github.com/jekyllbot" class="user-mention">@jekyllbot</a>`
- [ ] @mentions inside `<code>` or `<pre>` blocks are NOT converted
- [ ] @mentions inside existing `<a>` tags are NOT converted
- [ ] Email addresses (`user@example.com`) are NOT converted
- [ ] Custom base_url from config is respected
- [ ] DTC DOM match count must not drop below 790/790
- [ ] jekyll-docs maintaining/special-labels page @mention diffs resolved

## Test Scenarios

### Unit: @mention replacement

- `@jekyllbot` -> `<a href="https://github.com/jekyllbot" class="user-mention">@jekyllbot</a>`
- `user@example.com` -> NOT replaced (email address)
- `@username` inside `<code>` -> NOT replaced
- `@username` inside `<a href="...">` -> NOT replaced
- Multiple @mentions in one paragraph all replaced

### Unit: Plugin detection and config

- Site with `plugins: [jekyll-mentions]` -> enabled
- Site without jekyll-mentions -> no processing
- Site with `jekyll-mentions: { base_url: "https://gitlab.com" }` -> uses custom URL

### Integration: jekyll-docs site

- Build jekyll-docs, verify special-labels page has `<a class="user-mention">` links
- Run DOM comparison, verify no regression
