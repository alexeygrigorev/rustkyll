# Issue 519: jekyll-docs SEO meta description contains raw markdown links

## Problem

On 8 jekyll-docs pages, the SEO `<meta name="description">` and JSON-LD
`description` fields contain raw markdown link syntax (e.g.
`[Buddy][buddy-homepage]`) instead of stripped plain text (e.g. `Buddy`).

Jekyll's `jekyll-seo-tag` plugin calls `strip_html | normalize_whitespace |
escape` on the page excerpt before placing it into meta tags. Our SEO tag
implementation receives the excerpt but does not strip markdown reference-style
links from it before rendering.

### Affected pages (8)

- docs/continuous-integration/buddyworks/index.html (3 meta diffs)
- docs/continuous-integration/circleci/index.html (3 meta diffs)
- docs/continuous-integration/github-actions/index.html (3 meta diffs)
- docs/continuous-integration/razorops/index.html (3 meta diffs)
- docs/continuous-integration/travis-ci/index.html (3 meta diffs)
- docs/maintaining/affinity-team-captain/index.html (2 meta diffs)
- docs/plugins/commands/index.html (meta diffs)
- docs/posts/index.html (meta diffs)

### Example

Expected (Jekyll):
```
content='Buddy is a Docker-based CI server that you can set up in 15-20 minutes...'
```

Actual (rustkyll):
```
content='[Buddy][buddy-homepage] is a [Docker][docker-homepage]-based CI server...'
```

## Root Cause

The page excerpt used for SEO/meta tags preserves raw markdown syntax. Jekyll's
SEO plugin renders the excerpt through the markdown engine first, then strips
HTML tags. Our SEO tag takes the front matter excerpt (or auto-generated
excerpt) without markdown rendering.

## Scope

Fix the SEO tag to render/strip markdown from the description before outputting
it into meta tags and JSON-LD. This should strip:
- Reference-style links: `[text][ref]` -> `text`
- Inline links: `[text](url)` -> `text`
- HTML entities should be properly escaped

## Dependencies

- Issue 500 (jekyll-docs feed/meta/SEO fixes) should be done first or coordinated

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] Meta description tags contain plain text, not markdown syntax
- [ ] JSON-LD description contains plain text, not markdown syntax
- [ ] Reference-style links `[text][ref]` are stripped to `text`
- [ ] Inline links `[text](url)` are stripped to `text`
- [ ] HTML entities (`&amp;`) are preserved correctly in meta content
- [ ] DTC DOM match count must not drop below 790/790
- [ ] jekyll-docs DOM match count improves (8 pages fixed)

## Test Scenarios

### Unit: Markdown stripping for SEO descriptions

- Input `[Buddy][buddy-homepage] is great` -> output `Buddy is great`
- Input `[CircleCI](https://circleci.com) works` -> output `CircleCI works`
- Input with `&amp;` entities preserved correctly
- Input with no markdown links passes through unchanged

### Integration: jekyll-docs site

- Build jekyll-docs, check buddyworks page meta description is plain text
- Build jekyll-docs, check circleci page meta description is plain text
- Verify JSON-LD description matches meta description content
