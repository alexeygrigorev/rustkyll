# Issue 75: Fix raw Liquid tags in feed.xml content

## Problem

Discovered in issue #63 feed/sitemap validation: the DTC feed.xml contains raw Liquid tags (`{{`, `{%`) in entry content. The feed structure is correct (20 entries, valid Atom XML, correct titles/dates/links), but the `<content>` or `<summary>` elements contain unrendered Liquid template syntax.

This suggests that post body HTML is not being fully rendered through the Liquid engine before being included in the feed.

## Evidence

From `docs/comparison/feed-sitemap-results.md`:
- DTC feed.xml: 20 entries with valid structure
- Raw Liquid tags found in feed content (test `test_dtc_feed_validation` fails on Liquid tag check)

## Acceptance Criteria

- [ ] Build the DTC site; `feed.xml` contains no raw Liquid tags (`{{`, `{%`)
- [ ] `test_dtc_feed_validation` from `tests/integration_feed_sitemap.rs` passes (including the Liquid tag assertion)
- [ ] Feed entry content is fully rendered HTML, not raw template source

## Dependencies

- Issue #63 (feed/sitemap validation tests) -- provides the test that verifies this
