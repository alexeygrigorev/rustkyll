# Issue 522: jekyll-docs @mention plugin converts @username to GitHub links

## Problem

On 3 jekyll-docs pages, `@username` patterns (e.g. `@jekyllbot`) appear as
literal text instead of being converted to GitHub profile links. Jekyll uses the
`jekyll-mentions` plugin to convert `@username` into
`<a href="https://github.com/username" class="user-mention">@username</a>`.

### Affected pages (3) with current diff counts

- docs/maintaining/special-labels/index.html (43 diffs)
- docs/maintaining/triaging-an-issue/index.html (36 diffs)
- docs/maintaining/affinity-team-captain/index.html (31 diffs)

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

Rustkyll does not implement the `jekyll-mentions` plugin. The jekyll-docs site
has `jekyll-mentions` in its `plugins` list (line 49 of
`websites/jekyll-docs/docs/_config.yml`). This plugin runs as a post-render hook
that scans rendered HTML for `@username` patterns and replaces them with GitHub
profile links.

The implementation should go in the post-rendering pipeline. After markdown is
rendered to HTML and the layout is applied, scan text nodes for `@username`
patterns and replace them. This is similar to how other Jekyll plugins operate
as content transformers.

The relevant entry point is likely in `src/generator.rs` where page content is
assembled, or as a new post-processing step applied to final HTML.

## Scope

Implement basic jekyll-mentions-compatible @mention replacement:
1. Detect when `jekyll-mentions` is in the site's `plugins` or `gems` list
2. After HTML rendering, scan text nodes for `@username` patterns
3. Replace with `<a href="https://github.com/username" class="user-mention">@username</a>`
4. Do NOT replace inside `<code>`, `<pre>`, or `<a>` tags
5. Do NOT replace email addresses (`user@example.com`)
6. Support configurable `base_url` via `jekyll-mentions.base_url` in `_config.yml` (default: `https://github.com`)

Username pattern: `@` followed by one or more alphanumeric characters or hyphens,
not preceded by another alphanumeric character (to avoid matching email addresses).

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790 (DTC), jekyll-docs 14/125 matched
- Must not drop below: 790/790 (DTC)
- DTC does not use jekyll-mentions, so DTC output must be completely unaffected

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] @mentions are only processed when `jekyll-mentions` is in plugins/gems list
- [ ] `@jekyllbot` renders as `<a href="https://github.com/jekyllbot" class="user-mention">@jekyllbot</a>`
- [ ] @mentions inside `<code>` or `<pre>` blocks are NOT converted
- [ ] @mentions inside existing `<a>` tags are NOT converted
- [ ] Email addresses (`user@example.com`) are NOT converted
- [ ] Custom `base_url` from `jekyll-mentions` config is respected
- [ ] DTC DOM match count stays at 790/790 (DTC does not use this plugin)
- [ ] jekyll-docs maintaining/special-labels page diffs drop (currently 43)
- [ ] No regression on any other page or site
- [ ] Tests include non-ASCII content around @mentions (e.g., `Benutzer @jekyllbot hat...`)

## Test Scenarios

### Unit: @mention replacement

- `@jekyllbot` in plain text -> `<a href="https://github.com/jekyllbot" class="user-mention">@jekyllbot</a>`
- `user@example.com` -> NOT replaced (email address pattern)
- `@username` inside `<code>@username</code>` -> NOT replaced
- `@username` inside `<pre>` block -> NOT replaced
- `@username` inside `<a href="...">@username</a>` -> NOT replaced
- Multiple @mentions in one paragraph: `@alice and @bob` -> both replaced
- `@hyphenated-name` -> replaced (hyphens allowed in GitHub usernames)
- Unicode context: `Erstellt von @jekyllbot fur` -> `@jekyllbot` replaced, surrounding text preserved

### Unit: Plugin detection and config

- Site config with `plugins: [jekyll-mentions]` -> mentions processing enabled
- Site config with `gems: [jekyll-mentions]` -> mentions processing enabled (legacy key)
- Site config without jekyll-mentions -> no processing at all
- Site config with `jekyll-mentions: { base_url: "https://gitlab.com" }` -> uses `https://gitlab.com/username` URLs

### Integration: jekyll-docs site

- Build jekyll-docs, verify docs/maintaining/special-labels/index.html contains `<a class="user-mention">` links
- Build jekyll-docs, verify diff count drops on the 3 maintaining pages
- Build DTC, verify 790/790 DOM match (no change -- DTC does not use mentions)
- Run full DOM comparison, verify no regression on any site

### Output Verification

- Build jekyll-docs site with rustkyll
- Inspect generated HTML for docs/maintaining/special-labels/index.html
- Compare `@jekyllbot` occurrences against Jekyll reference at `websites/jekyll-docs/docs/_site_jekyll_cached/docs/maintaining/special-labels/index.html`
