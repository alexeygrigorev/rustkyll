# Issue 63: RSS/Atom feed and sitemap validation

## Problem

Issues #49 required RSS/Atom feed validation and sitemap comparison but these were never tested. The existing unit tests in `src/feed.rs` and `src/sitemap.rs` test the generation logic in isolation but never compare the actual output against what Jekyll produces for real sites. We need validation tests that build the sites, generate the XML files, and compare them against Jekyll reference output.

## Goal

Validate that rustkyll produces correct RSS/Atom feeds and sitemaps that match Jekyll's output for real sites. This means:
1. Building each site with rustkyll and inspecting the generated XML files
2. Building each site with Jekyll and comparing the output
3. Verifying XML validity, entry counts, URL correctness, and content accuracy

## Scope

### DataTalksClub/datatalksclub.github.io

- **feed.xml**: DTC uses the `jekyll-feed` plugin which generates an Atom feed. Rustkyll generates its own Atom feed via `src/feed.rs`. Compare entry count, titles, URLs, and dates.
- **sitemap.xml**: DTC has a custom Liquid-based `sitemap.xml` template (not the jekyll-sitemap plugin). It lists the root URL, standalone pages (people, articles, slack, events, books, podcast), plus all posts, people, books, conferences, and podcast items. Compare URL lists.

### alexeygrigorev/kids-horror-stories-ru

- **podcast.xml**: This site has a custom `podcast.xml` RSS feed (not Atom) for podcast episodes. It is rendered as a Liquid template with `layout: null`. Rustkyll must render this template correctly and produce valid RSS XML.
- **sitemap.xml**: This site does NOT have a sitemap template or jekyll-sitemap plugin. If rustkyll generates one programmatically, that is acceptable -- but we need to verify it does not crash and if generated, it is valid XML.

## Sites to validate

- DataTalksClub/datatalksclub.github.io (at `datatalksclub.github.io/` and `websites/DataTalksClub/datatalksclub.github.io/`)
- alexeygrigorev/kids-horror-stories-ru (at `websites/alexeygrigorev/kids-horror-stories-ru/`)

## Acceptance Criteria

### Feed validation -- DTC site

- [ ] Build the DTC site with rustkyll; a `feed.xml` file is generated in the output directory
- [ ] The generated `feed.xml` is valid XML (parses without errors using an XML parser)
- [ ] The feed contains the Atom namespace declaration (`xmlns="http://www.w3.org/2005/Atom"`)
- [ ] The feed `<title>` matches the site title from `_config.yml` (i.e., "DataTalks.Club")
- [ ] The feed `<link rel="self">` points to the correct feed URL
- [ ] The feed `<link rel="alternate">` points to the site root URL
- [ ] The feed contains at least 1 `<entry>` element (posts exist in DTC)
- [ ] Each `<entry>` has `<title>`, `<link>`, `<published>`, `<updated>`, and `<id>` child elements
- [ ] Feed entry count matches Jekyll's feed entry count (within 5% tolerance, or exact if both default to 20 posts)
- [ ] Feed entry titles match Jekyll's feed entry titles (same set of post titles)
- [ ] Feed entry URLs are valid and use the correct site URL prefix (`https://datatalks.club`)
- [ ] Feed entry dates are in RFC 3339 / ISO 8601 format

### Feed validation -- kids-horror-stories-ru

- [ ] Build the kids-horror-stories-ru site with rustkyll; a `podcast.xml` file is generated in the output directory (this is a Liquid template, not the programmatic feed generator)
- [ ] The generated `podcast.xml` is valid XML (parses without errors)
- [ ] The podcast.xml contains RSS 2.0 structure (`<rss version="2.0">`, `<channel>`, `<item>` elements)
- [ ] The podcast.xml `<item>` count matches Jekyll's output (within 5% tolerance)
- [ ] Each `<item>` has `<title>` and `<description>` child elements
- [ ] The podcast title, author, and description fields are populated (not empty or containing raw Liquid tags)

### Sitemap validation -- DTC site

- [ ] Build the DTC site with rustkyll; a `sitemap.xml` file is generated in the output directory
- [ ] The generated `sitemap.xml` is valid XML (parses without errors)
- [ ] The sitemap contains the sitemap namespace (`xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"`)
- [ ] The sitemap lists the root URL (`https://datatalks.club/`)
- [ ] The sitemap URL count matches Jekyll's sitemap URL count (within 5% tolerance)
- [ ] All sitemap URLs use the correct site URL prefix (`https://datatalks.club`)
- [ ] No sitemap URL contains raw Liquid tags (`{{`, `{%`)
- [ ] At least 90% of sitemap URLs that end in `.html` correspond to actual generated HTML files in the output directory (no broken internal URLs)
- [ ] The sitemap contains URLs for key sections: people, books, podcast, posts (at least one URL from each category)

### Sitemap validation -- kids-horror-stories-ru

- [ ] Build the kids-horror-stories-ru site with rustkyll; verify whether a `sitemap.xml` is generated
- [ ] If a sitemap.xml is generated, it is valid XML
- [ ] If a sitemap.xml is generated, its URLs correspond to actual generated files (no broken URLs)

### Test infrastructure

- [ ] Validation tests are implemented as `#[ignore]` integration tests (they require building real sites, so they must not run in the default test suite)
- [ ] Tests can be run with `cargo test --test <test_file> -- --ignored`
- [ ] Tests produce clear output indicating what was compared and the result (pass/fail with counts)

### Code quality

- [ ] `./scripts/cargo-safe test` passes (all existing tests still pass)
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] No `unwrap()` in library code (test code is fine)
- [ ] Results are documented in a markdown file (e.g., `docs/comparison/feed-sitemap-results.md`)

## Test Scenarios

### Integration: DTC feed.xml validation (ignored)
- Build DTC site with rustkyll
- Parse the generated `feed.xml` as XML; assert it parses successfully
- Count `<entry>` elements; assert count > 0
- Extract all entry titles; assert they are non-empty strings
- Extract all entry URLs; assert they start with `https://datatalks.club`
- Extract all entry dates; assert they match RFC 3339 format (`YYYY-MM-DDTHH:MM:SS`)
- Build DTC site with Jekyll (or use pre-built Jekyll output)
- Compare rustkyll feed entry count vs Jekyll feed entry count; assert within 5% tolerance
- Compare the set of entry titles; report any differences

### Integration: DTC feed.xml vs Jekyll comparison (ignored)
- Parse both rustkyll and Jekyll `feed.xml` files
- Extract entry titles from both; compute set intersection and differences
- Assert at least 90% of Jekyll's entries appear in rustkyll's feed (accounting for the max_posts limit)
- Extract entry URLs from both; assert URL patterns match (same path structure)

### Integration: kids-horror-stories-ru podcast.xml validation (ignored)
- Build kids-horror-stories-ru with rustkyll
- Parse the generated `podcast.xml` as XML; assert it parses successfully
- Assert it contains `<rss>` root element with version="2.0"
- Count `<item>` elements; assert count > 0
- Assert each `<item>` has a non-empty `<title>`
- Assert no raw Liquid tags appear in the output (`{{`, `{%`)
- Build with Jekyll and compare `<item>` counts; assert within 5%

### Integration: DTC sitemap.xml validation (ignored)
- Build DTC site with rustkyll
- Parse the generated `sitemap.xml` as XML; assert it parses successfully
- Count `<url>` elements; assert count > 50 (DTC has hundreds of pages)
- Extract all `<loc>` values; assert they all start with `https://datatalks.club`
- Assert no `<loc>` contains raw Liquid tags
- For each `<loc>` that ends in `.html`, strip the URL prefix and check the corresponding file exists in the output directory; assert at least 90% exist

### Integration: DTC sitemap.xml vs Jekyll comparison (ignored)
- Build DTC with both Jekyll and rustkyll
- Parse both `sitemap.xml` files
- Extract URL sets from both
- Compute intersection, rustkyll-only URLs, Jekyll-only URLs
- Assert total URL counts are within 5% tolerance
- Report the specific URLs that differ

### Integration: kids-horror-stories-ru sitemap validation (ignored)
- Build kids-horror-stories-ru with rustkyll
- Check if `sitemap.xml` exists in output; if yes, parse and validate it
- If it exists, verify URLs correspond to generated files

### Unit: XML parsing utility
- Write a helper that parses XML and extracts elements by tag name
- Test it with a minimal Atom feed string; verify correct extraction of entries
- Test it with a minimal sitemap string; verify correct extraction of URLs
- Test it with invalid XML; verify it returns an error, not a panic

## Dependencies

- Issue #16 (sitemap generation) -- done
- Issue #17 (RSS feed) -- done
- The DTC site source must be available at `datatalksclub.github.io/` or `websites/DataTalksClub/datatalksclub.github.io/`
- The kids-horror-stories-ru site source must be available at `websites/alexeygrigorev/kids-horror-stories-ru/`
- Jekyll must be installed to generate reference output for comparison (or pre-built Jekyll output must exist)

## Notes

- The DTC `sitemap.xml` is a custom Liquid template, not the programmatic one from `src/sitemap.rs`. Rustkyll may render the Liquid template OR use its programmatic generator -- either approach is acceptable as long as the output matches Jekyll's.
- The kids-horror-stories-ru `podcast.xml` is also a Liquid template with `layout: null`. It uses `absolute_url` filter, `site.podcast` config data, `site.stories` collection, and various filters. Correct rendering depends on the template engine.
- The `#[ignore]` attribute is required for all tests that build real sites, per project convention -- the default test suite must stay fast.
- The 5% tolerance accounts for minor differences in how rustkyll handles edge cases (e.g., draft posts, pages with `published: false`).
