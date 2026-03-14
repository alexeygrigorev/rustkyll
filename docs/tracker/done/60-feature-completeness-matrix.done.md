# Issue 60: Feature completeness matrix

## Problem

There is no single place to see which Jekyll features rustkyll supports and which are missing. Users need to quickly assess whether rustkyll will work for their site.

## Goal

Create a comprehensive feature completeness table comparing rustkyll against Jekyll. Publish it as a standalone page and link from the README.

## Deliverables

1. `docs/jekyll-compatibility.md` -- a table listing every Jekyll feature with its status in rustkyll
2. README.md updated with a link to the compatibility page

## Feature categories to cover

- Core: config parsing, front matter, Markdown rendering, layouts, includes, static files, permalinks
- Collections: posts, custom collections, drafts, pagination
- Templates: Liquid tags, filters, variables (site, page, content, paginator)
- Data files: YAML, JSON, CSV
- Plugins: jekyll-seo-tag, jekyll-feed, jekyll-sitemap, jekyll-redirect-from, jekyll-paginate, jekyll-avatar, jekyll-mentions, jekyll-include-cache, etc.
- Assets: Sass/SCSS, CoffeeScript
- CLI: build, serve, new, doctor, clean
- Other: incremental builds, live reload, baseurl/url handling, categories/tags, related_posts

## Table format

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| YAML front matter | yes | yes | |
| Sass/SCSS | yes | no | Pre-compile CSS as workaround |
| jekyll-paginate | yes | no | |

Use "yes", "partial", or "no" for status.

## Dependencies

None

## Acceptance Criteria

- [ ] `docs/jekyll-compatibility.md` exists and is valid Markdown (renders correctly on GitHub)
- [ ] The document is organized into clearly labeled sections matching the categories above (Core, Collections, Templates, Data files, Plugins, Assets, CLI, Other)
- [ ] Each section contains a Markdown table with columns: Feature, Jekyll, rustkyll, Notes
- [ ] Every major Jekyll feature is listed -- at minimum 50 distinct feature rows across all categories
- [ ] Status values are strictly one of: "yes", "partial", or "no"
- [ ] Every "yes" or "partial" status is verifiable by finding the corresponding implementation in `src/` (the engineer must cite the source file or module in the Notes column or in a summary)
- [ ] Every "no" status is accurate -- the feature genuinely has no implementation in the codebase
- [ ] "partial" is used when a feature exists but has known limitations; the Notes column explains what is missing
- [ ] The document includes a summary section at the top showing total counts: N yes, N partial, N no
- [ ] `README.md` contains a link to `docs/jekyll-compatibility.md` (use a relative path)
- [ ] No changes to any files under `src/`, `tests/`, or `scripts/`
- [ ] The Liquid filters section lists individual filters (not just "filters" as one row) -- at least the following: date, where, group_by, sort, size, strip_html, xml_escape, url_encode, slugify, jsonify, array_to_sentence_string, markdownify, smartify, relative_url, absolute_url, default, first, last, join, map, concat, push, replace, split, strip, downcase, upcase, capitalize, truncate, truncatewords, escape, newline_to_br, number_of_words, plus, minus, times, divided_by, modulo, append, prepend, remove, remove_first, replace_first
- [ ] The Liquid tags section lists individual tags -- at least: if/elsif/else/endif, for/endfor, assign, capture, include, comment, raw, highlight, unless, case/when

## Test Scenarios

Since this is a documentation-only issue with no code changes, there are no `cargo test` scenarios. Verification is manual/review-based.

### Verification: Accuracy audit (performed by tester and PM)

- Pick 5 features marked "yes" at random. Grep the codebase for the corresponding implementation. Confirm each actually exists.
- Pick 3 features marked "no" at random. Grep the codebase to confirm there is no implementation.
- Pick all features marked "partial". Read the Notes column. Confirm the limitation described is real (not fabricated).

### Verification: Completeness check

- Cross-reference the Jekyll documentation (https://jekyllrb.com/docs/) categories against the sections in the compatibility doc. Confirm no major category is omitted.
- Verify the done issues in `docs/tracker/done/` are reflected in the matrix -- every feature implemented via a tracker issue should appear as "yes" or "partial" in the matrix.

### Verification: README link

- Open `README.md` and confirm a working relative link to `docs/jekyll-compatibility.md` exists.
- The link text should make clear what the document is (e.g., "Jekyll Compatibility" or "Feature Comparison").

### Verification: Formatting

- The Markdown renders as valid tables (no broken pipes, consistent column counts).
- The summary counts at the top match the actual counts in the tables below.
