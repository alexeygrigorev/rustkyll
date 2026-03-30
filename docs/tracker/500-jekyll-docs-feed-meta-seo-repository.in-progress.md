# Issue 500: jekyll-docs feed_meta, SEO tag, and site.repository fixes

## Problem

jekyll-docs is at 16/125 (13%). Every page has DOM differences caused by:

1. `feed_meta` tag generates category-specific feed links that real Jekyll's feed_meta does NOT generate (only the main feed link)
2. `site.repository` config value not exposed to Liquid context (parsed by config but not injected into site object)
3. SEO tag reads `site.title` but does not fall back to `site.name` (Jekyll's jekyll-seo-tag does)
4. SEO tag does not output `google-site-verification` meta tag

## Acceptance Criteria

- feed_meta outputs ONLY the main feed link, not category-specific links
- site.repository is available in Liquid templates
- SEO tag title falls back to site.name when site.title is empty
- SEO tag outputs google-site-verification when configured
- DTC DOM stays at 790/790
- jekyll-docs DOM improves from 16/125

## Constraints

- Do NOT modify generator.rs or collection.rs
