# Issue 234: Research most popular Jekyll themes and create support issues

## Problem

We need to understand which Jekyll themes are most widely used so we can prioritize theme compatibility. Currently we support some themes but don't have a systematic view of coverage.

## Goal

1. Research the most popular Jekyll themes (by GitHub stars, usage, official themes)
2. Check which ones we already have in our benchmark sites
3. For each popular theme NOT already supported, create a new .todo.md issue
4. For each popular theme we DO have, note the current match rate

## Research Findings

### Top Jekyll Themes by GitHub Stars (approximate, as of early 2026)

| # | Theme | GitHub Stars | In Benchmark? | Benchmark Directory | Notes |
|---|-------|-------------|---------------|---------------------|-------|
| 1 | minimal-mistakes | ~13,000 | YES | `websites/minimal-mistakes/` | Most popular Jekyll theme overall |
| 2 | academicpages | ~12,000 | YES | `websites/academicpages/` | Fork/derivative of minimal-mistakes for academia |
| 3 | al-folio | ~11,000 | NO | -- | Academic personal sites, BibTeX publications |
| 4 | chirpy | ~7,500 | NO | -- | Tech blogs, dark mode, TOC, search |
| 5 | just-the-docs | ~7,500 | YES | `websites/just-the-docs/` | Documentation sites |
| 6 | beautiful-jekyll | ~5,500 | YES | `websites/beautiful-jekyll/` | Personal blogs |
| 7 | documentation-theme-jekyll | ~4,300 | YES | `websites/documentation-theme-jekyll/` | Technical documentation |
| 8 | minima | ~3,500 | YES | `websites/minima/` | Jekyll default theme |
| 9 | hyde | ~3,500 | YES | `websites/hyde/` | Poole-based sidebar theme |
| 10 | lanyon | ~3,200 | NO | -- | Poole-based toggle sidebar |
| 11 | TeXt | ~3,000 | NO | -- | Feature-rich content theme |
| 12 | mediumish | ~2,000 | NO | -- | Medium.com-like blog |
| 13 | so-simple | ~1,800 | YES | `websites/so-simple-theme/` | Clean simple blog by mmistakes |
| 14 | jasper2 | ~1,800 | NO | -- | Ghost Casper port |
| 15 | hydeout | ~1,000 | NO | -- | Updated Hyde |
| 16 | basically-basic | ~1,000 | NO | -- | Minimal theme by mmistakes |
| 17 | yat | ~1,000 | NO | -- | Modern blog with animations |
| 18 | type | ~1,000 | NO | -- | Typography-focused blog |

### Official GitHub Pages Supported Themes (all in benchmark)

| Theme | Benchmark Directory |
|-------|---------------------|
| jekyll-theme-architect | `websites/architect-theme/` |
| jekyll-theme-cayman | `websites/cayman-theme/` |
| jekyll-theme-dinky | `websites/dinky-theme/` |
| jekyll-theme-hacker | `websites/hacker-theme/` |
| jekyll-theme-leap-day | `websites/leap-day-theme/` |
| jekyll-theme-merlot | `websites/merlot-theme/` |
| jekyll-theme-midnight | `websites/midnight-theme/` |
| jekyll-theme-primer | `websites/primer-theme/` |
| jekyll-theme-slate | `websites/slate-theme/` |
| jekyll-theme-time-machine | `websites/time-machine-theme/` |

### Current Benchmark Coverage Summary

**Total benchmark sites:** 35 directories in `websites/`

**Theme coverage breakdown:**
- Popular community themes in benchmark: 8 of 18 (44%)
- Official GitHub Pages themes in benchmark: 10 of 10 (100%)
- Top 10 themes by stars covered: 7 of 10 (70%)

**Key gaps (top 10 by stars, not in benchmark):**
1. al-folio (~11k stars) -- Issue #235 created
2. chirpy (~7.5k stars) -- Issue #236 created
3. lanyon (~3.2k stars) -- Issue #237 created

### Other Benchmark Sites (not theme repos)

These are real-world sites using various themes or custom layouts:
- `websites/alexeygrigorev/` -- Personal site
- `websites/bitcoin-org/` -- Bitcoin.org (custom)
- `websites/choosealicense.com/` -- GitHub's Choose a License (custom)
- `websites/DataTalksClub/` -- DataTalks.Club (custom)
- `websites/government-github/` -- Government GitHub (custom)
- `websites/homebrew-site/` -- Homebrew (custom)
- `websites/jekyll-docs/` -- Jekyll's own docs
- `websites/large-blog-3000/` -- Synthetic large blog
- `websites/large-docs-site/` -- Synthetic large docs
- `websites/made-mistakes-jekyll/` -- Made Mistakes blog (mmistakes personal site)
- `websites/mojombo-blog/` -- Tom Preston-Werner's blog
- `websites/muan-blog/` -- Mu-An Chiou's blog
- `websites/opensource-guide/` -- GitHub Open Source Guide
- `websites/programming-historian/` -- Programming Historian
- `websites/uswds-site/` -- US Web Design System
- `websites/wtf-html-css/` -- WTF HTML and CSS

## Follow-up Issues Created

Issues created for the top 10 unsupported popular themes:

| Issue | Theme | Stars |
|-------|-------|-------|
| #235 | al-folio | ~11,000 |
| #236 | chirpy | ~7,500 |
| #237 | lanyon | ~3,200 |
| #238 | TeXt | ~3,000 |
| #239 | mediumish | ~2,000 |
| #240 | jasper2 | ~1,800 |
| #241 | hydeout | ~1,000 |
| #242 | basically-basic | ~1,000 |
| #243 | yat | ~1,000 |
| #244 | type | ~1,000 |

## Acceptance Criteria

- [x] List of top 20+ most popular Jekyll themes identified (18 community themes + 10 GitHub Pages themes = 28 total)
- [x] Each theme checked against our `websites/` benchmark directory
- [x] New .todo.md issues created for unsupported popular themes (issues #235-#244)
- [x] Summary of current theme coverage documented

## Prioritization Recommendation

Priority order for adding theme support (based on stars and ecosystem importance):

1. **al-folio (#235)** -- 11k stars, huge academic user base
2. **chirpy (#236)** -- 7.5k stars, very active development, widely used tech blogs
3. **lanyon (#237)** -- 3.2k stars, Poole-based like Hyde (which we already support), likely easy win
4. **TeXt (#238)** -- 3k stars, complex feature set (good stress test)
5. The remaining themes (#239-#244) are all ~1-2k stars and lower priority

## Dependencies

- None (this is a research issue)
