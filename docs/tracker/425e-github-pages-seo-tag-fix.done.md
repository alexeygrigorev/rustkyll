# Issue 425E: Fix GitHub Pages theme SEO tag generation

## Problem

10 GitHub Pages gem theme sites (architect, cayman, dinky, hacker, leap-day,
merlot, midnight, primer, slate, time-machine) are all at 0/2 DOM match due to
SEO tag differences in `<head>`.

## Root Causes

1. **Canonical URL / og:url emitted without site.url**: When `site.url` is not
   configured, Jekyll's `jekyll-seo-tag` does NOT emit `<link rel="canonical">`
   or `<meta property="og:url">`. Rustkyll was incorrectly falling back to
   page_url as a relative canonical.

2. **JSON-LD script closing format**: Jekyll puts `}</script>` on the same line.
   Rustkyll was emitting `}\n</script>` (newline before closing tag), causing
   DOM comparison mismatches.

## Fix

- Removed canonical/og:url fallback when site.url is absent (seo_tag.rs lines 396-415)
- Changed JSON-LD output from `}\n</script>` to `}</script>` (seo_tag.rs line 591)

## Acceptance Criteria

- [x] DTC DOM stays at 790/790
- [x] All 10 GitHub Pages theme sites reach 2/2 (100%)
- [x] All existing tests pass
- [x] New tests for both fixes

## Log

### [SWE] 2026-03-28
- Investigated cayman-theme and architect-theme DOM diffs
- Root cause 1: canonical/og:url emitted when site.url absent (Jekyll does not do this)
- Root cause 2: JSON-LD closing tag on separate line from closing brace
- TDD: wrote 5 new failing tests (test_no_canonical_without_site_url, test_no_canonical_without_site_url_subpage, test_jsonld_script_closing_no_newline, test_canonical_emitted_with_site_url, test_jsonld_url_uses_page_url_when_no_site_url)
- Verified tests FAIL before fix
- Implemented fix in src/template/seo_tag.rs
- Updated 2 existing tests (test_jsonld_compact_single_line, test_jsonld_script_tag_format) to match new correct behavior
- All 118 SEO tag tests pass, full suite 3033+ tests pass, 0 failures
- Clippy clean, fmt clean
- DOM results:
  - DTC: 790/790 (unchanged)
  - All 10 theme sites: 2/2 (was 0/2)
- Files modified: src/template/seo_tag.rs
