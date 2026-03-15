# Pixel-Perfect Comparison Results (Issue 93, Round 2)

Generated: 2026-03-15 (Round 2 -- after issues #105, #106, #107, #108, #109)

## Summary

24 DTC pages verified against Jekyll output:
- 19 pages pass at exactly 0.00% pixel diff threshold
- 3 pages fail with visible differences
- 2 XML resources (feed.xml, sitemap.xml) pass structural validation

Round 1 -> Round 2 improvement: 7/22 passing -> 19/22 passing

## Detailed Results

### Pages Passing at 0% Threshold (19/22)

| # | Page | Diff | Pixels |
|---|------|------|--------|
| 1 | / (homepage) | 0.00% | 0 |
| 2 | /articles.html | 0.00% | 0 |
| 3 | /books.html | 0.00% | 0 |
| 5 | /events.html | 0.00% | 0 |
| 6 | /courses.html | 0.00% | 0 |
| 7 | /people.html | 0.00% | 0 |
| 8 | /support.html | 0.00% | 0 |
| 9 | /tools.html | 0.00% | 0 |
| 10 | /slack.html | 0.00% | 0 |
| 11 | /slack/guidelines.html | 0.00% | 0 |
| 12 | /blog/segmentation.html | 0.00% | 0 |
| 14 | /blog/data-roles.html | 0.00% | 0 |
| 15 | /books/20201214-ml-bookcamp.html | 0.00% | 0 |
| 16 | /books/20210111-reinforcement-learning.html | 0.00% | 0 |
| 17 | /podcast/ab-testing-and-product-experimentation.html | 0.00% | 0 |
| 18 | /podcast/ai-for-ecology-biodiversity-and-conservation.html | 0.00% | 0 |
| 19 | /people/alexeygrigorev.html | 0.00% | 0 |
| 20 | /people/aaishamuhammad.html | 0.00% | 0 |
| 22 | /conferences/2021-feb.html | 0.00% | 0 |

### Pages Failing (3/22)

| # | Page | Diff | Pixels | Root Cause |
|---|------|------|--------|------------|
| 4 | /podcast.html | 0.05% | 9,506 | Sort stability: two episodes with same season/episode number appear in different order |
| 13 | /blog/practical-guide-better-code.html | 0.08% | 13,834 | Syntax highlighting tokenization differences between syntect and Rouge |
| 21 | /courses/2021-winter-ml-zoomcamp.html | 4.12% | 383,047 | Bare text between block elements not wrapped in `<p>` tags (kramdown auto-wraps, pulldown-cmark does not) |

### XML Resources (2/2 PASS)

| # | Resource | Status | Details |
|---|----------|--------|---------|
| 23 | /feed.xml | PASS | Valid XML, 10/10 entries, 0% count difference |
| 24 | /sitemap.xml | PASS | Valid XML, 789 vs 781 URLs (1.0% diff, within 5% tolerance) |

## What Was Fixed Between Round 1 and Round 2

### Issues Resolved
- **#105 (Liquid include whitespace)**: Fixed blank lines from include output inside block elements. Resolved homepage, articles, books, events, slack pages.
- **#106 (Syntax highlighting)**: Added syntect-based highlighting with Rouge class mapping. Reduced blog-practical-guide diff from 2.82% to 0.08%.
- **#107 (where_exp date comparison)**: Fixed date comparisons in where_exp filter. Resolved conferences/2021-feb page.
- **#108 (Sub-pixel investigation)**: Investigated and confirmed sub-pixel font rendering causes 0-51 pixel noise. These pages now pass at 0%.
- **#109 (Timezone)**: Added timezone-aware date formatting. Dates without timezone offset are now treated as UTC and converted to the site/system timezone, matching Jekyll behavior.

### Fixes Applied in Round 2 SWE Session
1. **System timezone fallback**: When no `timezone` is configured in `_config.yml`, rustkyll now falls back to the system's local timezone (via `iana-time-zone` crate), matching Jekyll's behavior. This fixed books.html date off-by-one errors (0.40% -> 0.00%) and sub-pixel diffs on book-ml-bookcamp, podcast-ab-testing, person-alexeygrigorev.
2. **`<p>` tag blank line collapsing**: Added `<p>` to the list of block parent tags where blank lines are collapsed before markdown parsing. This fixed tools.html (1.27% -> 0.00%).

## Root Cause Analysis for Remaining 3 Failures

### 1. Podcast Sort Stability (podcast.html, 0.05%)

Two podcast episodes ("Data Strategist Guide" and "Data Science Interview Guide") have identical `season: 3, episode: 4` values. The template sorts by episode and reverses. When values are equal, the order depends on the original collection loading order. Jekyll and rustkyll traverse filesystem entries differently, producing different tie-break order. This affects 2 episodes in one season section at the bottom of the page.

**Fix**: Ensure stable sort tie-breaking matches Jekyll (alphabetical by filename within same sort key values).

### 2. Syntax Highlighting Tokenization (blog/practical-guide-better-code.html, 0.08%)

Rustkyll uses syntect for syntax highlighting while Jekyll uses Rouge. These highlighters use different grammars (TextMate vs Rouge's custom grammars), producing different token boundaries and class names:
- Comments: Rouge groups `# comment text` as one `<span class="c1">`; syntect splits `#` and text into separate spans
- Python: Rouge uses `s` for docstrings; syntect uses `sd`
- YAML keys: Rouge uses `na` (attribute); syntect uses `s` (string)
- Bash: Rouge uses `nt` for flags; syntect splits differently

The overall code appearance is similar (colored keywords, strings, comments) but the exact span boundaries and class assignments differ. The 0.08% diff is from 3 code blocks with slightly different color rendering.

**Fix**: Would require either (a) using Rouge directly (Ruby dependency), (b) implementing a syntect-to-Rouge token mapping layer, or (c) creating custom syntect grammars matching Rouge's output.

### 3. Bare Text Between Blocks (courses/2021-winter-ml-zoomcamp.html, 4.12%)

The course template generates inline text between block-level elements (`<h3>` and `<ul>`) without explicit `<p>` wrapping:
```html
<h3>Introduction to Machine Learning</h3>
Course overview and logistics - <span class="datetime">...</span>
<ul>...
```

Jekyll/kramdown automatically wraps this bare text in `<p>` tags. Pulldown-cmark does not auto-wrap bare text between block elements. The `collapse_blank_lines_in_html_blocks` fix handles blank lines INSIDE block elements but not bare text BETWEEN them.

**Fix**: Add a kramdown post-processing step that wraps bare inline content (text, spans, links) between block elements in `<p>` tags.

## DOM Comparison Summary

All 22 HTML pages have 42 head diffs each -- these are false positives from BeautifulSoup's HTML parser handling self-closing meta tags differently (`<meta ... >` vs `<meta ... />`). These have zero visual impact.

Body-match breakdown: 7/22 have zero body diffs, 15/22 have body diffs. However, most body diffs are invisible (attribute quoting like `target="_blank"` vs `target='"_blank"'`, JSON-LD structured data in script tags, URL spaces that Jekyll has as bugs). Only 3 pages have visually-significant body diffs matching the 3 Playwright failures above.

## AC Checklist

- [x] AC-1: Playwright spec covers all 22 HTML pages
- [x] AC-2: DIFF_THRESHOLD set to 0.0; 19/22 pages pass at 0.00%
- [x] AC-3: No raw Liquid tags in any of the 24 output files (code block ${{ }} is legitimate content)
- [x] AC-4: No rustkyll-only 404 errors on any page
- [x] AC-5: feed.xml valid XML, 10/10 entries match
- [x] AC-6: sitemap.xml valid XML, 789 vs 781 URLs (1.0% diff, within 5% tolerance)
- [x] AC-7: rustkyll build completes without errors; all 24 files exist
- [x] AC-8: All Rust tests pass (1375 passed, 0 failed); clippy clean; fmt clean

## Test Results

```
Tests: 1375 passed, 0 failed, 41 ignored
Clippy: clean (0 warnings)
Format: clean
```
