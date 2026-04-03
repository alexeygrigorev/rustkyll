# Issue 500: jekyll-docs feed_meta, SEO tag, and site.repository fixes

## Problem

jekyll-docs was at 16/125 (13%). Every page had DOM differences caused by:

1. `feed_meta` tag generated category-specific feed links that real Jekyll's feed_meta does NOT generate (only the main feed link)
2. `site.repository` config value not exposed to Liquid context (parsed by config but not injected into site object)
3. SEO tag reads `site.title` but did not fall back to `site.name` (Jekyll's jekyll-seo-tag does)
4. SEO tag did not output `google-site-verification` meta tag

## Acceptance Criteria

- [x] feed_meta outputs ONLY the main feed link, not category-specific links
- [x] site.repository is available in Liquid templates
- [x] SEO tag title falls back to site.name when site.title is empty
- [x] SEO tag outputs google-site-verification when configured
- [x] DTC DOM stays at or above 596/790 baseline
- [x] jekyll-docs DOM improved from 16/125

## Constraints

- Do NOT modify generator.rs or collection.rs

## Log

### [PM] 2026-04-02 16:00
- All 4 fixes were implemented in commit 209d360 (2026-03-29)
- Commit message: "Fix jekyll-docs: feed_meta, site.repository, SEO site.name, title-from-slug"
- Files changed: src/main.rs, src/template/feed_meta_tag.rs, src/template/seo_tag.rs, docs/dom-baselines.json
- feed_meta: verified code only emits main feed link, category feed link generation removed, test confirms no category links
- site.repository: verified injection at main.rs lines 558-565 via config.repository
- SEO site.name fallback: verified at seo_tag.rs -- site.title || site.name logic, with dedicated test
- google-site-verification: verified at seo_tag.rs lines 732-737, with dedicated test
- DOM verification: DTC 596/790 (matches baseline), jekyll-docs 22/125 (improved from original 16/125)
- Remaining jekyll-docs diffs are syntax highlighting class mismatches and timestamp diffs, unrelated to this issue
- VERDICT: ACCEPT -- all 4 fixes committed, tests present, no regressions
