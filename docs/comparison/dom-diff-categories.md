# DOM Diff Audit: Categorized Differences (DTC Site)

Generated: 2026-03-16 (post-fixes #137-146)
Jekyll site: `datatalksclub.github.io/_site`
Rustkyll site: `_site`

**Summary: 429 files matched, 358 files with differences, 4258 total differences**

Previous (pre-fixes): 323 matched, 464 with diffs, 4472 total diffs
Delta: +106 matched files (+32.8%), -214 total diffs (-4.8%)

## Categories

| # | Category | Count | Files | Example | Issue # | Status |
|---|----------|------:|------:|---------|---------|--------|
| 1 | Syntax highlighting token class mismatch | ~825 | 26 | `class='k'` vs `class='nb'` in `pre > code > span` | #113 | partial |
| 2 | Syntax highlighting text/span count | ~445 | ~10 | Missing/extra `<span>` in `pre > code`, mismatched highlight text | #113 | partial |
| 3 | Kramdown paragraph wrapping (list items, figcaption, blockquote) | ~687 | ~37 | `<li><p>text</p></li>` (Jekyll) vs `<li>text</li>` (rustkyll) | #124 | todo |
| 4 | Text content diffs (cascade from structural + real diffs) | ~1184 | 91 | Text nodes shifted because of missing/extra elements above | N/A | cascade |
| 5 | Missing `class='highlighter-rouge language-plaintext'` on inline `<code>` | 300 | 50 | Jekyll adds class, rustkyll does not | #145 | **reverted?** |
| 6 | JSON-LD text content diffs (dates, descriptions, misc) | 251 | 245 | Various small JSON-LD field differences | #137,#138,#142 | partial |
| 7 | Extra/missing text nodes | 163 | ~40 | Extra or missing text between elements | N/A | cascade |
| 8 | Extra JSON-LD script tags (books, some pages) | 100 | 99 | Rustkyll emits `<script type="application/ld+json">` where Jekyll has none | #139 | partial |
| 9 | Tag/element type mismatches | 62 | ~20 | Tag name differs at same position in tree | N/A | cascade |
| 10 | Heading ID: leading number stripped | ~42 | ~12 | `id='1-datatalksclub'` vs `id='datatalksclub'` | #141 | **regression** |
| 11 | JSON-LD timezone offset (+01:00/+02:00 vs +00:00) | 18 | 18 | `datePublished: "...+01:00"` vs `+00:00"` | #109 | partial |
| 12 | Extra paragraph/br | 18 | 10 | Extra `<p>` or `<br>` in rustkyll output | #148 | in-progress |
| 13 | Missing/extra misc elements | ~166 | ~60 | Various: li, ul, ol, em, a, h2, h3, code, div, pre, strong, img, figcaption, blockquote | #148 | in-progress |
| 14 | Heading ID: double-dash / ampersand | ~2 | ~2 | `id='devops--site-reliability-engineer'` vs single dash | #141 | mostly fixed |
| 15 | JSON-LD null vs empty string | 1 | 1 | `"datePublished": null` vs `""` | #142 | in-progress |

**Total accounted: ~4264 (of 4258 -- minor overlap in categorization)**

## What Changed (Fixes #137-146)

### Fixes that landed (done):
- **#137** (JSON-LD trailing newline): Reduced JSON-LD description diffs. Previous ~211 -> now folded into remaining 251 JSON-LD diffs.
- **#138** (JSON-LD podcast date format): Fixed `dateModified` format. Previous ~386 -> significantly reduced.
- **#139** (Extra JSON-LD script tags): Reduced from ~199 to 100. Some extra scripts remain.
- **#140** (Book listing end-date off-by-one): Fixed. Previous 78 diffs in 1 file -> eliminated.
- **#141** (Heading ID generation): Fixed double-dash and ampersand issues. But introduced regression: leading numbers in heading IDs are now stripped (`1-datatalksclub` -> `datatalksclub`). ~42 new diffs.
- **#143** (URL percent-encoding): Fixed. Previous 4 diffs -> eliminated.
- **#144** (Accordion script placement): Fixed.
- **#145** (Extra class on inline code): Appears partially reverted or not fully effective -- 300 diffs remain for missing `highlighter-rouge language-plaintext` class on inline `<code>` elements. Previously was 9 diffs (rustkyll had extra class), now 300 diffs (rustkyll is missing class). Direction flipped.
- **#146** (OL start attribute): Fixed. Previous 33 diffs -> eliminated.

### Still in-progress:
- **#142** (JSON-LD keyword types and null dates): 1 null-vs-empty diff remains.
- **#147** (Extra target=_blank): In progress.
- **#148** (Misc markdown rendering edge cases): ~166 misc element diffs remain.

## Top Remaining Diff Categories (by impact)

1. **Syntax highlighting** (~1270 diffs, 26 files): Token class mismatches and span count differences between Syntect (rustkyll) and Rouge (Jekyll). Concentrated in code-heavy blog posts. This is the single largest category.

2. **Text content diffs / cascade** (~1184 diffs, 91 files): Most of these are secondary effects from structural differences (paragraph wrapping, missing elements). When a `<p>` tag is missing, all subsequent text nodes appear shifted, generating many false-positive text diffs. Fixing #3 and #13 would eliminate most of these.

3. **Kramdown paragraph wrapping** (~687 diffs, ~37 files): Jekyll/Kramdown wraps content in `<p>` tags inside `<li>`, `<figcaption>`, and `<blockquote>` in certain contexts where rustkyll does not. This is issue #124 (still todo).

4. **Missing inline code class** (300 diffs, 50 files): Jekyll adds `class='highlighter-rouge language-plaintext'` to inline `<code>` elements. Rustkyll does not. Issue #145 was marked done but the fix appears to have the wrong direction (or was reverted).

5. **JSON-LD diffs** (~270 total: 251 text + 18 timezone + 1 null): Various remaining JSON-LD field differences including timezone offsets and minor text formatting.

6. **Extra JSON-LD script tags** (100 diffs, 99 files): Rustkyll emits JSON-LD script tags on pages where Jekyll does not.

7. **Missing/extra misc elements** (~166 diffs, ~60 files): Various edge cases in markdown rendering.

## Estimated Impact of Remaining Fixes

If the top 3 categories were fully fixed:
- Syntax highlighting (#113): ~1270 diffs
- Kramdown paragraph wrapping (#124) + cascade: ~687 + ~1000 cascade = ~1687 diffs
- Inline code class (#145): ~300 diffs

That would eliminate approximately 3257 of 4258 diffs (76%), bringing us to ~1000 remaining diffs.

## Top 10 Files by Diff Count

| File | Diffs | Primary cause |
|------|------:|---------------|
| blog/open-source-free-ai-agent-evaluation-tools.html | 550 | syntax highlighting |
| blog/important-sql-fact-that-everyone-should-know.html | 414 | syntax highlighting |
| blog/how-to-setup-lightweight-local-version-for-airflow.html | 411 | syntax highlighting |
| blog/ml-deployment-lambda.html | 316 | syntax highlighting |
| blog/practical-guide-better-code.html | 315 | syntax highlighting |
| blog/ner-reformers.html | 200 | syntax highlighting |
| blog/how-to-run-postgresql-and-pgadmin-with-docker.html | 197 | syntax highlighting |
| blog/how-do-data-professionals-use-data-engineering-tools-and-practices.html | 164 | mixed (stale content + structural) |
| blog/essentials-of-public-speaking-for-career-in-data-science.html | 63 | paragraph wrapping + cascade |
| blog/free-data-engineering-courses.html | 53 | paragraph wrapping + cascade |

Note: 204 files have exactly 1 diff (most commonly a single JSON-LD text difference).

## Reproduction

```bash
# Build both sites (assuming Jekyll site already at datatalksclub.github.io/_site)
./scripts/cargo-safe build --release && ./target/release/rustkyll build --source datatalksclub.github.io --destination _site

# Run DOM comparison (summary)
python3 scripts/dom_compare.py --jekyll-dir datatalksclub.github.io/_site --rustkyll-dir _site --output docs/comparison/dom-diff-current.txt

# Run full DOM comparison (all diffs, no truncation)
python3 scripts/dom_compare_full.py datatalksclub.github.io/_site _site docs/comparison/dom-diff-full-report-current.txt

# Categorize results
python3 scripts/categorize_diffs.py docs/comparison/dom-diff-full-report-current.txt
```
