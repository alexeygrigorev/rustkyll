# DOM Diff Audit: Categorized Differences (DTC Site)

Generated: 2026-03-16
Jekyll site: `datatalksclub.github.io/_site`
Rustkyll site: `_site`

**Summary: 323 files matched, 464 files with differences, 4472 total differences**

## Categories

| # | Category | Count | Files | Example | Issue # | Status |
|---|----------|------:|------:|---------|---------|--------|
| 1 | Syntax highlighting token class mismatch | 852 | 8 | `class='k'` vs `class='nb'` in `pre > code > span` | #113 | partial |
| 2 | Syntax highlighting text/span count | 299+43 = 342 | 8 | Missing/extra `<span>` in `pre > code`, mismatched highlight text | #113 | partial |
| 3 | Kramdown paragraph wrapping in list items | 512+356+260 = ~1128 | 42 | `<li><p>text</p></li>` (Jekyll) vs `<li>text</li>` (rustkyll) | #124 | todo |
| 4 | JSON-LD timezone offset (+01:00/+02:00 vs +00:00) | 106 | ~100 | `datePublished: "2023-12-11T00:00:00+01:00"` vs `+00:00` | #109 | partial |
| 5 | JSON-LD trailing newline in description | 211 | ~200 | `"description": "...\n"` vs `"...\n\n"` | #137 | **new** |
| 6 | JSON-LD date format (podcast dateModified/startDate/endDate) | 386 | ~200 | `"dateModified": "2026-03-16 08:23:32 +0100"` vs `"2026-03-16 08:30:38 +0000"` | #138 | **new** |
| 7 | Extra JSON-LD script tag (books, some pages) | 98+101 = ~199 | 100 | Rustkyll emits `<script type="application/ld+json">` where Jekyll has none | #139 | **new** |
| 8 | Book listing end-date off-by-one | 78 | 1 | `"to 11 Oct 2025"` vs `"to 10 Oct 2025"` | #140 | **new** |
| 9 | Heading ID double-dash collapsing | 12 | 3 | `id='devops--site-reliability-engineer'` vs `id='devops-site-reliability-engineer'` | #141 | **new** |
| 10 | Heading ID ampersand encoding in anchors | 7 | 2 | `id='free--free-to-audit-courses'` vs `id='free-amp-free-to-audit-courses'` | #141 | **new** |
| 11 | JSON-LD string-vs-number in keywords | 5 | 2 | `"keywords": ["2024"]` vs `"keywords": [2024]` | #142 | **new** |
| 12 | JSON-LD null vs empty string for dates | 2 | 1 | `"datePublished": null` vs `""` | #142 | **new** |
| 13 | Entity encoding in JSON-LD `name` field | 13 | 2 | `"AI for Testing, CI/CD & DevOps"` vs `"CI/CD &amp; DevOps"` | #102 | partial |
| 14 | URL encoding in thumbnailUrl/image (space vs %20) | 4 | 1 | `hybrid%20search.jpg` vs `hybrid search.jpg` | #143 | **new** |
| 15 | Accordion script src attribute mismatch | 9 | 4 | `src='/assets/accordion.js'` missing or on wrong element | #144 | **new** |
| 16 | Extra `class` on inline code elements | 9 | 3 | Rustkyll adds `class='highlighter-rouge language-plaintext'` to `<code>` | #145 | **new** |
| 17 | `start` attribute on ordered lists | 33 | ~5 | Rustkyll adds `start='2'` etc. to `<ol>` where Jekyll doesn't | #146 | **new** |
| 18 | Extra `target='_blank'` on links | 3 | 2 | Rustkyll adds `target='_blank'` where Jekyll doesn't | #147 | **new** |
| 19 | Stale content (one blog post with different title/dates/content) | ~55 | 1 | `how-do-data-professionals-use-data-engineering-tools-and-practices.html` has completely different content | N/A | build-order issue |
| 20 | Podcast transcript text differences | 59 | ~30 | Minor whitespace diffs in long transcript text | #137 | **new** |
| 21 | Cascade text diffs (from structural mismatches) | ~400 | 96 | Text nodes shifted because of missing/extra elements above | N/A | resolves when structural issues are fixed |
| 22 | Missing/extra misc elements (figcaption, blockquote, img, h1-h3, etc.) | ~50 | ~20 | Various edge-case markdown rendering differences | #148 | **new** |

## Notes

### Cascade effects

Categories 3 (kramdown paragraph wrapping) and 21 (cascade text diffs) are closely related. When rustkyll omits `<p>` wrappers inside `<li>`, `<figcaption>`, or `<blockquote>`, the DOM comparison reports the missing `<p>` as one diff, then all subsequent text nodes appear shifted, generating many secondary `text_differs` and `tag_name_differs` entries. Fixing category 3 should eliminate most of category 21.

### Stale content

Category 19 is NOT a rustkyll bug. The file `how-do-data-professionals-use-data-engineering-tools-and-practices.html` was apparently updated in the Jekyll source after the rustkyll build was done, so the content simply differs because of different build times. A clean rebuild from the same source will eliminate this.

### Syntax highlighting

Categories 1-2 (852+342 = ~1194 diffs) are ALL in just 8 files. These are pages with code blocks where Syntect (rustkyll) tokenizes differently from Rouge (Jekyll). Issue #113 made progress but did not fully resolve all token mapping differences.

### Estimated impact of fixes

If the top 5 categories were fixed:
- Category 3 (kramdown wrapping): ~1128 diffs + ~400 cascade = ~1528
- Category 1-2 (syntax highlighting): ~1194 diffs
- Category 4-6 (JSON-LD dates): ~703 diffs
- Category 7 (extra JSON-LD scripts): ~199 diffs

That would eliminate approximately 3624 of 4472 diffs (81%).

## Reproduction

```bash
# Build both sites (assuming Jekyll site already at datatalksclub.github.io/_site)
./scripts/cargo-safe build --release && ./target/release/rustkyll build --source datatalksclub.github.io --destination _site

# Run full DOM comparison
python3 scripts/dom_compare_full.py datatalksclub.github.io/_site _site docs/comparison/dom-diff-full-report.txt

# Categorize results
python3 scripts/categorize_diffs.py docs/comparison/dom-diff-full-report.txt
```
