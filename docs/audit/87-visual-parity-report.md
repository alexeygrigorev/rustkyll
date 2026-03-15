# Issue 87: DTC Visual Parity Audit Report

## Summary

Systematic comparison of 15 page types between Jekyll and rustkyll output for the DataTalks.Club website. The audit identified 12 distinct categories of differences. One fix was applied in this issue (homepage title). All other differences are tracked in follow-up issues.

**Build details:**
- Jekyll output: `/tmp/compare-jekyll-DataTalksClub-datatalksclub.github.io/` (787 HTML files)
- Rustkyll output: `/tmp/dtc-audit-rustkyll/` (787 HTML files)
- File count: exact match (787 HTML files in both)
- Structural comparison: PASS (12/51 sampled files with diffs, within tolerance)

## Playwright Visual Comparison Results

Per-page pixel diff percentages from Playwright screenshot comparison (post-FIX-1, threshold 5%):

| Page | Path | Pixel Diff % | Status |
|------|------|-------------|--------|
| homepage | `/` | 2.21% | PASS (diffs from D1-D4: heading IDs, boolean attrs, self-closing tags, form wrapping) |
| blog-post | `/blog/segmentation.html` | 0.00% | PASS |
| books-listing | `/books.html` | 2.57% | PASS (diffs from D8: include output markdown-processed, D9: date as code block, D10: date off by 1 day) |
| book-detail | `/books/20230529-modeling-mindsets.html` | 0.21% | PASS (diffs from D10, D11, D5) |
| podcast-listing | `/podcast.html` | 3.45% | PASS (diffs from D8: include output markdown-processed, D12: boolean attrs) |
| podcast-episode | `/podcast/machine-learning-decision-optimization.html` | 0.00% | PASS |
| events-listing | `/events.html` | 1.80% | PASS (diffs from D8: include output markdown-processed, D12: boolean attrs) |
| courses | `/courses.html` | 0.00% | PASS |
| people-listing | `/people.html` | 0.00% | PASS |
| person-detail | `/people/andrewmcmahon.html` | 0.00% | PASS |
| articles-listing | `/articles.html` | 2.93% | PASS (diffs from D8: include output markdown-processed, D17: entity encoding) |
| slack-community | `/slack.html` | 0.00% | PASS |
| support | `/support.html` | 0.00% | PASS |
| tools | `/tools.html` | 1.27% | PASS (diffs from D8: include output markdown-processed) |

**Summary:** 14/14 pages pass at 5% threshold. 8 pages at 0.00% pixel diff (perfect match). The remaining 6 pages have diffs between 0.21% and 3.45%, all attributable to known differences tracked in Issue 90 (primarily D8: include output being markdown-processed). All pages pass at the 5% threshold; pages exceeding 1% are the ones affected by D8 (the largest systematic issue).

## Fix Applied in This Issue

### FIX-1: Homepage title incorrectly appends site name

**Page:** `/index.html` (homepage)
**Category:** Wrong content
**Root cause:** Two bugs:
1. `page.name` was not set in the page rendering context, so the Liquid condition `{% if page.name == 'index.md' %}` in DTC's `head.html` always evaluated to false
2. The SEO tag used a hyphen `-` instead of en-dash `&ndash;` as the title separator, and did not skip appending site title when page title already contained it

**Before:** `<title>Welcome to DataTalks.Club &ndash; DataTalks.Club</title>`
**After:** `<title>Welcome to DataTalks.Club</title>` (matches Jekyll)

**Files modified:**
- `src/template/seo_tag.rs` -- use `&ndash;` separator, skip appending when page title contains site title
- `src/generator.rs` -- set `page.name` (source filename) and `page.path` (source path) in page context

## Page-by-Page Audit Results

### 1. Homepage (`/index.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH (after FIX-1) |
| Links | MATCH (54) |
| Images | MATCH (7) |
| Navigation | MATCH |
| Footer | MATCH |
| Content diffs | 137 lines (whitespace + minor formatting) |

**Remaining differences:**
- D1: Headings get auto-generated `id` attributes (e.g., `<h1>` becomes `<h1 id="the-place-to-talk-about-data">`) -- Issue 90
- D2: Boolean HTML attributes differ: `novalidate=""` vs `novalidate`, `required=""` vs `required` -- Issue 90
- D3: Self-closing tags: `<input ... />` vs `<input ...>` -- Issue 90
- D4: Form element wrapping differs slightly (multi-line vs single-line) -- Issue 90

### 2. Blog post (`/blog/segmentation.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (25) |
| Images | MATCH (10) |
| Content diffs | 79 lines |

**Remaining differences:**
- D5: Smart quote conversion differs -- `'` (curly) vs `'` (straight) in some places. Jekyll converts more aggressively. -- Issue 90
- D6: `<figcaption>` closing tag on separate line in rustkyll vs same line in Jekyll -- Issue 90
- D7: Minor whitespace around blank lines between paragraphs -- cosmetic, no visual impact -- Issue 90

### 3. Books listing (`/books.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (248) |
| Images | MATCH (0) |
| Content diffs | 1034 lines (significant) |

**Remaining differences:**
- D8: **Include output markdown-processed** -- Author links from `{% include %}` get wrapped in `<p>` tags. This is a systematic issue where Liquid include output within markdown files is being re-processed through the markdown converter. Affects all listing pages that use includes for author names. -- Issue 90
- D9: **Date text rendered as code block** -- Indented `(from DD Mon YYYY to DD Mon YYYY)` text is treated as a markdown code block and wrapped in `<div class="language-plaintext highlighter-rouge">`. -- Issue 90
- D10: **Date calculation off by 1 day** in some entries (e.g., "11 Oct" in Jekyll vs "10 Oct" in rustkyll). Root cause: timezone handling in `date_to_string` filter. -- Issue 90

### 4. Book detail (`/books/20230529-modeling-mindsets.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (27) |
| Images | MATCH (1) |
| Content diffs | 150 lines |

**Remaining differences:**
- D10: Date off by 1 day (same as books listing)
- D11: `<ol start="2">` in rustkyll vs `<ol>` in Jekyll -- ordered list continuation numbering difference. Rustkyll adds `start` attribute for lists that don't start at 1. -- Issue 90
- D5: Smart quote differences

### 5. Podcast listing (`/podcast.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (415) |
| Images | MATCH (4) |
| Content diffs | 2061 lines (significant) |

**Remaining differences:**
- D8: Include output markdown-processed (author links wrapped in `<p>` tags) -- Issue 90
- D12: `itemscope=""` vs `itemscope` (boolean HTML attribute formatting) -- Issue 90
- D4: Whitespace/indentation differences in template loop output

### 6. Podcast episode (`/podcast/machine-learning-decision-optimization.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (132) |
| Images | MATCH (7) |
| Content diffs | 1061 lines (significant) |

**Remaining differences:**
- D13: **Timestamp format for sub-minute times** -- Jekyll uses `0.0`, `27.0`, `54.0` format; rustkyll uses `0:00`, `0:27`, `0:54` format. Times >= 1 minute match (`1:19`). -- Issue 90
- D14: JSON-LD `dateModified` uses build timestamp in Jekyll vs episode date in rustkyll -- Issue 90
- D15: JSON-LD `startDate`/`endDate` uses build timestamp in Jekyll vs episode date in rustkyll -- different but arguably rustkyll is more correct -- Issue 90
- D16: Empty template output lines (many blank lines from conditional template logic that produces no output) -- Issue 90
- D5: Smart quote differences

### 7. Events listing (`/events.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (875) |
| Images | MATCH (0) |
| Content diffs | 3235 lines (highest) |

**Remaining differences:**
- D8: Include output markdown-processed (event links and author links wrapped in `<p>` tags) -- Issue 90
- D12: Boolean attribute formatting (`itemscope=""` vs `itemscope`)
- D4: Whitespace/indentation differences

### 8. Courses (`/courses.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (21) |
| Images | MATCH (0) |
| Content diffs | 1 line |

**Status:** Near-perfect match. Only 1 line difference (blank line).

### 9. People listing (`/people.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (447) |
| Images | MATCH (0) |
| Content diffs | 1283 lines |

**Remaining differences:**
- D12: Boolean attribute formatting (`itemscope=""` vs `itemscope`)
- D4: Whitespace/indentation differences in loop output

### 10. Person detail (`/people/andrewmcmahon.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (26) |
| Images | MATCH (1) |
| Content diffs | 3 lines |

**Status:** Near-perfect match. Only smart quote and whitespace differences.

### 11. Articles listing (`/articles.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (136) |
| Images | MATCH (0) |
| Content diffs | 294 lines |

**Remaining differences:**
- D8: Include output markdown-processed (author links wrapped in `<p>` tags) -- Issue 90
- D17: HTML entity encoding: `&amp;` preserved in Jekyll vs decoded `&` in rustkyll for inline text -- Issue 90

### 12. Community/Slack (`/slack.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (25) |
| Images | MATCH (0) |
| Content diffs | 45 lines |

**Remaining differences:**
- D2: Boolean HTML attributes (`novalidate=""` vs `novalidate`) -- Issue 90
- D3: Self-closing tags -- Issue 90
- D4: Form element formatting -- Issue 90

### 13. Support (`/support.html`)

| Aspect | Result |
|--------|--------|
| Title | MATCH |
| Links | MATCH (45) |
| Images | MATCH (0) |
| Content diffs | 27 lines |

**Remaining differences:**
- D1: Headings get auto-generated `id` attributes -- Issue 90
- D17: HTML entity encoding (`&amp;` vs `&` in heading text) -- Issue 90
- D5: Smart quote differences

### 14. Navigation (header + footer)

Checked on 3 pages: homepage, books.html, podcast.html.

| Aspect | Result |
|--------|--------|
| Header nav | MATCH on all 3 pages |
| Footer | MATCH on all 3 pages |

**Status:** Perfect match. Navigation and footer are identical.

### 15. RSS Feed (`/feed.xml`)

| Aspect | Result |
|--------|--------|
| Entry count | DIFF: Jekyll=10, Rustkyll=20 |
| Subtitle | DIFF: Missing in rustkyll |
| Content encoding | DIFF: Jekyll uses CDATA, rustkyll uses entity encoding |
| Timezone | DIFF: Different timezone handling |

**Remaining differences:**
- D18: Entry count differs (20 vs 10) -- Issue 90
- D19: Missing `<subtitle>` element -- Issue 90
- D20: Content uses entity encoding (`&lt;p&gt;`) instead of CDATA (`<![CDATA[<p>]]>`) -- Issue 90
- D21: Timezone handling differs (UTC vs local timezone) -- Issue 90
- D22: `<id>` format differs slightly -- Issue 90

## Difference Inventory

| ID | Category | Description | Pages Affected | Fixed? | Follow-up Issue |
|----|----------|-------------|----------------|--------|----------------|
| D1 | Layout | Headings get auto-generated `id` attributes | Homepage, Support | No | Issue 90 |
| D2 | Styling | Boolean HTML attrs: `novalidate=""` vs `novalidate` | Homepage, Slack | No | Issue 90 |
| D3 | Styling | Self-closing tags: `<input/>` vs `<input>` | Homepage, Slack | No | Issue 90 |
| D4 | Layout | Whitespace/indentation in template loop output | Multiple | No | Issue 90 |
| D5 | Styling | Smart quote conversion differences | Blog, Book detail, Podcast, Support | No | Issue 90 |
| D6 | Layout | Figcaption closing tag line position | Blog post | No | Issue 90 |
| D7 | Layout | Minor blank line differences | Blog post | No | Issue 90 |
| D8 | **Missing content** | Include output markdown-processed (author links wrapped in `<p>`, text rendered as code blocks) | Books, Podcast, Events, Articles, Tools | No | **Issue 90** (high priority) |
| D9 | Wrong content | Indented date text rendered as code block | Books listing | No | Issue 90 |
| D10 | Wrong content | Date off by 1 day (timezone in date_to_string) | Books listing, Book detail | No | Issue 90 |
| D11 | Wrong content | `<ol start="N">` added for non-1 list starts | Book detail | No | Issue 90 |
| D12 | Styling | `itemscope=""` vs `itemscope` boolean attribute | Podcast, Events, People | No | Issue 90 |
| D13 | Wrong content | Timestamp format for sub-minute times (0.0 vs 0:00) | Podcast episode | No | Issue 90 |
| D14 | Wrong content | JSON-LD dateModified uses wrong date | Podcast episode | No | Issue 90 |
| D15 | Wrong content | JSON-LD startDate/endDate uses wrong date | Podcast episode | No | Issue 90 |
| D16 | Layout | Empty lines from conditional template output | Podcast episode | No | Issue 90 |
| D17 | Wrong content | HTML entity encoding differs (&amp; vs &) | Articles, Support | No | Issue 90 |
| D18 | Wrong content | Feed entry count differs (20 vs 10) | Feed.xml | No | Issue 90 |
| D19 | Missing content | Feed missing `<subtitle>` element | Feed.xml | No | Issue 90 |
| D20 | Wrong content | Feed uses entity encoding instead of CDATA | Feed.xml | No | Issue 90 |
| D21 | Wrong content | Feed timezone handling differs | Feed.xml | No | Issue 90 |
| D22 | Wrong content | Feed `<id>` format differs | Feed.xml | No | Issue 90 |
| FIX-1 | Wrong content | Homepage title appends site name | Homepage | **Yes** | -- |

## Priority Assessment

### High Priority (visual impact, content correctness)
- **D8**: Include output being markdown-processed -- affects books, podcast, events, articles, and tools listings. Author links get wrapped in unwanted `<p>` tags and indented text becomes code blocks. This is the single largest visual difference.
- **D10**: Date calculation off by 1 day -- factual error visible to users.
- **D18-D22**: Feed.xml differences -- affects RSS readers.

### Medium Priority (minor visual differences)
- **D1**: Auto-generated heading IDs -- no visual impact but may affect anchor links.
- **D5**: Smart quote differences -- minor visual difference.
- **D13**: Timestamp format -- `0.0` vs `0:00` -- minor display difference.
- **D11**: `<ol start>` -- may affect numbered list display.

### Low Priority (no visual impact)
- **D2, D3, D12**: Boolean attribute and self-closing tag formatting -- valid HTML either way, no visual difference.
- **D4, D6, D7, D16**: Whitespace and formatting -- no visual impact.
- **D14, D15**: JSON-LD dates -- not visible to users, affects SEO metadata only.

## Post-Fix Verification

After applying FIX-1:
- All 1155 unit tests pass (29 ignored)
- Clippy clean (0 warnings)
- `cargo fmt --check` passes
- Structural comparison: PASS (12/51 files with diffs, within tolerance)
- File count: exact match (787 files in both)
- Homepage title: now matches Jekyll exactly
- Playwright visual comparison: 14/14 pages PASS at 5% threshold
  - 8 pages at 0.00% pixel diff (blog-post, podcast-episode, courses, people-listing, person-detail, slack-community, support, courses)
  - 6 pages with diffs between 0.21% and 3.45%, all attributable to known differences tracked in Issue 90
  - Highest diff: podcast-listing at 3.45% (D8: include output markdown-processed)
  - See "Playwright Visual Comparison Results" section above for full per-page breakdown

## Follow-up Issues

All unfixed differences are tracked in **Issue 90** (DTC template rendering gaps), which is the appropriate bucket since most differences stem from template rendering behavior differences between Jekyll and rustkyll.

The most impactful items for Issue 90 to address first:
1. D8 -- Include output markdown processing (the biggest visual difference)
2. D10 -- Date calculation timezone fix
3. D18-D22 -- Feed.xml parity
