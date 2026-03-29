# Issue 521: jekyll-docs jemoji emoji shortcode rendering

## Problem

On 5+ jekyll-docs pages, emoji shortcodes like `:heart:`, `:smiley:`,
`:white_check_mark:`, and `:x:` appear as literal text instead of being converted
to `<img>` tags (or Unicode emoji). Jekyll uses the `jemoji` plugin to convert
GitHub-style emoji shortcodes into images.

### Affected pages (5+)

- docs/maintaining/affinity-team-captain/index.html
- docs/maintaining/becoming-a-maintainer/index.html
- docs/maintaining/merging-a-pull-request/index.html
- docs/maintaining/reviewing-a-pull-request/index.html
- docs/security/index.html (`:white_check_mark:` and `:x:` in table cells -> `<img>`)
- docs/maintaining/triaging-an-issue/index.html (`:christmas_tree:`)

### Example

Expected (Jekyll with jemoji):
```html
<p>Your work means the world to our thousands of users. <img class="emoji" title=":heart:" alt=":heart:" src="https://github.githubassets.com/images/icons/emoji/unicode/2764.png" height="20" width="20"></p>
```

Actual (rustkyll):
```html
<p>Your work means the world to our thousands of users. :heart:</p>
```

In tables, `:white_check_mark:` and `:x:` should become `<img>` elements but
remain as text.

## Root Cause

Rustkyll does not implement the `jemoji` Jekyll plugin. This plugin scans rendered
HTML for `:shortcode:` patterns and replaces them with GitHub emoji image tags.

## Scope

Implement basic jemoji-compatible emoji shortcode replacement:
1. Detect when `jemoji` is in the site's `plugins` list (or `gems` for older configs)
2. After markdown rendering, scan text nodes for `:shortcode:` patterns
3. Replace with `<img>` tags pointing to GitHub emoji assets

The full jemoji plugin supports all GitHub emoji. For this issue, support at least
the emoji used in jekyll-docs: `:heart:`, `:smiley:`, `:smile:`, `:tada:`,
`:sparkles:`, `:confetti_ball:`, `:white_check_mark:`, `:x:`, `:christmas_tree:`.

A lookup table mapping shortcodes to Unicode codepoints is sufficient.

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] Emoji shortcodes are only processed when jemoji is in plugins list
- [ ] `:heart:` renders as an `<img>` tag with correct src/alt/title
- [ ] `:white_check_mark:` and `:x:` in table cells render as `<img>` tags
- [ ] Shortcodes inside `<code>` or `<pre>` blocks are NOT converted
- [ ] Unknown shortcodes are left as-is (no error)
- [ ] DTC DOM match count must not drop below 790/790
- [ ] jekyll-docs maintaining pages emoji diffs resolved

## Test Scenarios

### Unit: Emoji shortcode replacement

- `:heart:` -> `<img class="emoji" ...>` with correct Unicode src
- `:white_check_mark:` -> `<img>` with checkmark emoji
- `:unknown_emoji:` -> left as `:unknown_emoji:` (no crash)
- `:heart:` inside `<code>` -> NOT replaced (left as text)
- Multiple emoji in one paragraph all replaced

### Unit: Plugin detection

- Site with `plugins: [jemoji]` in config -> emoji processing enabled
- Site with `gems: [jemoji]` in config -> emoji processing enabled
- Site without jemoji -> no emoji processing

### Integration: jekyll-docs site

- Build jekyll-docs, verify affinity-team-captain page has `<img>` emoji
- Build jekyll-docs, verify security page table has `<img>` for checkmarks
- Run DOM comparison, verify no regression
