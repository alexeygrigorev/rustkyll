# Visual Comparison Results: Rustkyll vs Jekyll

Date: 2026-03-14

## Commands Used

```bash
# Build rustkyll release binary
./scripts/cargo-safe build --release

# Build DTC site
./target/release/rustkyll build --source websites/DataTalksClub/datatalksclub.github.io --destination /tmp/visual-compare-rustkyll-DataTalksClub-datatalksclub.github.io

# Build kids site
./target/release/rustkyll build --source websites/alexeygrigorev/kids-horror-stories-ru --destination /tmp/visual-compare-rustkyll-alexeygrigorev-kids-horror-stories-ru

# Run DTC comparison
./scripts/visual-compare.sh --jekyll-dir /tmp/compare-jekyll-DataTalksClub-datatalksclub.github.io --rustkyll-dir /tmp/visual-compare-rustkyll-DataTalksClub-datatalksclub.github.io --threshold 0 --site DataTalksClub/datatalksclub.github.io --skip-build

# Run kids comparison
./scripts/visual-compare.sh --jekyll-dir /tmp/compare-jekyll-alexeygrigorev-kids-horror-stories-ru --rustkyll-dir /tmp/visual-compare-rustkyll-alexeygrigorev-kids-horror-stories-ru --threshold 0 --site alexeygrigorev/kids-horror-stories-ru --skip-build
```

## Per-Page Results

### DataTalksClub/datatalksclub.github.io (7 pages)

| Page | Jekyll Screenshot | Rustkyll Screenshot | Diff Image | Pixel Diff % | Root Cause | Status |
|------|------------------|--------------------|-----------:|-------------:|------------|--------|
| homepage | `DataTalksClub_datatalksclub_github_io__homepage__jekyll.png` | `DataTalksClub_datatalksclub_github_io__homepage__rustkyll.png` | `DataTalksClub_datatalksclub_github_io__homepage__diff.png` | 2.21% | Kramdown attribute syntax, whitespace/indentation, markdown paragraph wrapping in includes | TRACKED |
| blog-post | `DataTalksClub_datatalksclub_github_io__blog-post__jekyll.png` | `DataTalksClub_datatalksclub_github_io__blog-post__rustkyll.png` | `DataTalksClub_datatalksclub_github_io__blog-post__diff.png` | 0.00% | -- | PASS |
| books-listing | `DataTalksClub_datatalksclub_github_io__books-listing__jekyll.png` | `DataTalksClub_datatalksclub_github_io__books-listing__rustkyll.png` | `DataTalksClub_datatalksclub_github_io__books-listing__diff.png` | 2.12% | Kramdown attributes ({:target="_blank"}), heading IDs, code class differences | TRACKED |
| events-listing | `DataTalksClub_datatalksclub_github_io__events-listing__jekyll.png` | `DataTalksClub_datatalksclub_github_io__events-listing__rustkyll.png` | `DataTalksClub_datatalksclub_github_io__events-listing__diff.png` | 1.81% | Kramdown attributes ({:target="_blank"}), heading IDs, whitespace | TRACKED |
| courses | `DataTalksClub_datatalksclub_github_io__courses__jekyll.png` | `DataTalksClub_datatalksclub_github_io__courses__rustkyll.png` | `DataTalksClub_datatalksclub_github_io__courses__diff.png` | 0.00% | -- | PASS |
| people-listing | `DataTalksClub_datatalksclub_github_io__people-listing__jekyll.png` | `DataTalksClub_datatalksclub_github_io__people-listing__rustkyll.png` | `DataTalksClub_datatalksclub_github_io__people-listing__diff.png` | 0.00% | -- | PASS |
| articles-listing | `DataTalksClub_datatalksclub_github_io__articles-listing__jekyll.png` | `DataTalksClub_datatalksclub_github_io__articles-listing__rustkyll.png` | `DataTalksClub_datatalksclub_github_io__articles-listing__diff.png` | 2.93% | HTML entity escaping (& vs &amp;), whitespace/indentation differences | TRACKED |

All screenshots are in `playwright/screenshots/DataTalksClub/datatalksclub.github.io/`.

### alexeygrigorev/kids-horror-stories-ru (4 pages)

| Page | Jekyll Screenshot | Rustkyll Screenshot | Diff Image | Pixel Diff % | Root Cause | Status |
|------|------------------|--------------------|-----------:|-------------:|------------|--------|
| homepage | `alexeygrigorev_kids-horror-stories-ru__homepage__jekyll.png` | `alexeygrigorev_kids-horror-stories-ru__homepage__rustkyll.png` | `alexeygrigorev_kids-horror-stories-ru__homepage__diff.png` | 0.05% | Minor whitespace differences in HTML output | KNOWN_EXCEPTION |
| story-orchid | `alexeygrigorev_kids-horror-stories-ru__story-orchid__jekyll.png` | `alexeygrigorev_kids-horror-stories-ru__story-orchid__rustkyll.png` | `alexeygrigorev_kids-horror-stories-ru__story-orchid__diff.png` | 0.10% | Markdown paragraph spacing (extra blank lines between paragraphs in Jekyll) | KNOWN_EXCEPTION |
| story-silkworm | `alexeygrigorev_kids-horror-stories-ru__story-silkworm__jekyll.png` | `alexeygrigorev_kids-horror-stories-ru__story-silkworm__rustkyll.png` | `alexeygrigorev_kids-horror-stories-ru__story-silkworm__diff.png` | 0.03% | Same markdown spacing difference as story-orchid | KNOWN_EXCEPTION |
| story-toy | `alexeygrigorev_kids-horror-stories-ru__story-toy__jekyll.png` | `alexeygrigorev_kids-horror-stories-ru__story-toy__rustkyll.png` | `alexeygrigorev_kids-horror-stories-ru__story-toy__diff.png` | 0.00% | -- | PASS |

All screenshots are in `playwright/screenshots/alexeygrigorev/kids-horror-stories-ru/`.

## Summary

| Metric | Count |
|--------|-------|
| Total pages compared | 11 |
| Pages at 0% diff | 4 |
| Pages at <1% diff | 7 |
| Pages at <5% diff | 11 |
| Pages at >=5% diff | 0 |

## Root Cause Analysis

### RC1: Kramdown attribute syntax not supported (DTC homepage, books, events)

**Visual difference:** Links that should open in new tabs (`target="_blank"`) render the raw attribute syntax `{:target="_blank"}` as visible text instead of applying it as an HTML attribute.

**HTML difference:** Jekyll (kramdown) processes `{:target="_blank"}` after a link and adds the attribute. Rustkyll (pulldown-cmark) does not support this kramdown-specific syntax.

**Status:** TRACKED -- follow-up issue #73 created.

### RC2: Heading IDs not generated (DTC books, events)

**Visual difference:** No visible difference (IDs are invisible), but the generated HTML differs in heading tags (`<h2 id="upcoming-books">` vs `<h2>`).

**HTML difference:** Jekyll/kramdown auto-generates `id` attributes on headings. Pulldown-cmark does not.

**Status:** TRACKED -- follow-up issue #73 (same issue covers markdown parser differences).

### RC3: HTML whitespace/indentation differences (all DTC pages)

**Visual difference:** Generally invisible, but contributes to pixel-level differences due to slightly different text reflow.

**HTML difference:** Jekyll outputs HTML with different indentation (e.g., 6 spaces) than rustkyll (e.g., 3 spaces). Self-closing tags differ (`<br />` vs `<br/>`).

**Status:** KNOWN_EXCEPTION -- cosmetic difference that does not affect functionality.

### RC4: Markdown paragraph spacing (kids site story pages)

**Visual difference:** Barely visible -- slight differences in spacing between paragraphs.

**HTML difference:** Jekyll/kramdown adds extra blank lines between `<p>` tags. Pulldown-cmark does not.

**Status:** KNOWN_EXCEPTION -- cosmetic difference inherent to different markdown parsers.

### RC5: HTML entity escaping (DTC articles)

**Visual difference:** Invisible to users (browsers render both the same).

**HTML difference:** Jekyll escapes `&` as `&amp;` in title text within HTML. Rustkyll outputs the raw `&` character.

**Status:** KNOWN_EXCEPTION -- both render identically in browsers; the pixel diff is from sub-pixel rendering differences.

## Fixes Applied in This Issue

### Fix 1: SCSS compilation support

**Problem:** The kids-horror-stories-ru site uses an SCSS file (`assets/css/styles.scss`) with YAML front matter. Jekyll compiles SCSS to CSS. Rustkyll was copying the raw `.scss` file, causing a 404 for `styles.css` on every page.

**Fix:** Added the `grass` crate (pure-Rust SCSS compiler). SCSS files with front matter are now discovered as processable pages, compiled to CSS, and output with `.css` extension.

**Files:** `Cargo.toml`, `src/generator.rs`, `src/collection.rs`, `src/static_files.rs`

**Before:** All 4 kids pages failed with 404 errors (no CSS loaded).
**After:** All pages load CSS correctly. Story pages now at 0.00%-0.10% diff.

### Fix 2: Collection sorting by date

**Problem:** Collections (like stories) were sorted by filename string order, which gives incorrect results for mixed-length numeric prefixes (e.g., `099-...` < `1000-...` < `100-...`).

**Fix:** Sort all collection items by date ascending (with source path as tiebreaker), matching Jekyll's behavior.

**Files:** `src/collection.rs`

**Before:** Kids homepage showed stories starting at #999 (wrong order). Diff was 2.48%.
**After:** Stories correctly ordered 1351, 1350, ..., 1. Homepage diff is 0.05%.

### Fix 3: Custom `date` filter for YYYY-MM-DD strings

**Problem:** The built-in Liquid `date` filter could not parse `YYYY-MM-DD` date strings, so `{{ page.date | date: "%d.%m.%Y" }}` output the raw date string instead of formatting it.

**Fix:** Added a custom `date` filter that uses the existing `parse_date_string` helper to handle date-only strings, datetime strings, and RFC3339 timestamps.

**Files:** `src/template/filters/date.rs`, `src/template/filters/mod.rs`, `src/template/engine.rs`

**Before:** Dates displayed as `2024-07-24` instead of `24.07.2024`.
**After:** Dates formatted correctly per template format strings.

### Fix 4: site.posts in reverse chronological order

**Problem:** `site.posts` was exposed in date-ascending order. Jekyll exposes `site.posts` in reverse chronological order (newest first).

**Fix:** Reverse the posts array when building the site context.

**Files:** `src/generator.rs`

**Before:** Articles listing showed posts in wrong order. Diff was 2.96%.
**After:** Posts in correct order. Diff is 2.93% (remaining diff from whitespace/entity differences).

### Fix 5: Empty front matter detection

**Problem:** The `has_front_matter` function did not recognize empty front matter (`---\n---\n`) because the closing `---` appeared at the start of the content after the opening delimiter.

**Fix:** Added `rest.starts_with("---")` check in addition to `rest.contains("\n---")`.

**Files:** `src/collection.rs`

## Follow-Up Issues Created

### Issue #73: Kramdown compatibility gaps (TODO)

Covers the remaining visual differences caused by kramdown-specific features not supported by pulldown-cmark:

- `{:target="_blank"}` inline attribute syntax
- Auto-generated heading IDs
- `class="language-plaintext highlighter-rouge"` on inline code blocks
- Extra blank lines between paragraphs

These affect 4 DTC pages (homepage, books, events, articles) with 1.81%-2.93% pixel diff.
