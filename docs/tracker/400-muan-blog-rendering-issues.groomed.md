# Issue 400: muan-blog -- multiple rendering issues (2199/2218)

## Problem

muan-blog is at 2199/2218 (99.1%) DOM match with 19 pages differing. Multiple
root causes affect different pages. This is a parent/tracking issue to coordinate
fixes across sub-issues.

### Known sub-categories

1. **iframe/img wrapped in `<p>` tags** (9 diffs across ~7 pages) -- tracked as #449
2. **Heading ID generation** (multiple pages) -- heading IDs differ from Jekyll output
   (e.g., `id='codeltdetailsgtcode-basics'` vs `id='details-basics'`)
3. **Datetime formatting** -- ISO 8601 format vs Jekyll's `datetime` attribute format
   (e.g., `2013-05-21T11:02:39+08:00` vs `2013-05-21T03:02:39+08:00`, timezone offset)
4. **URL encoding of Unicode** -- Unicode characters in URLs handled differently
5. **Text content differences** -- some link text or paragraph text split differently
   (e.g., mailto: links with special characters)
6. **`notes.html` list page** (24 diffs) -- structural differences in notes listing
7. **`posts/border-box-in-github.html`** (34 diffs) -- extra `<br>` elements in
   blockquotes, structural reordering

### Pages with diffs (from DOM comparison, partial list)

- `notes.html` (24 diffs)
- `posts/border-box-in-github.html` (34 diffs)
- `posts/presence.html` (3 diffs -- img in p)
- `posts/acceptance.html` (1 diff -- iframe in p)
- `posts/mission-focused.html` (1 diff -- iframe in p)
- `posts/details-on-details.html` (3 diffs -- iframe in p + heading IDs)
- `posts/leaving-github.html` (4 diffs -- iframe in p + link text)
- `posts/noise.html` (3 diffs -- iframe in p + datetime + missing br)
- Various `notes/` pages (2-14 diffs each)

## Scope

This is a **tracking issue**. The engineer must:
1. Rebuild muan-blog with current code
2. Run DOM comparison and categorize all remaining diffs
3. For each category, either fix it directly (if small) or create a focused sub-issue
4. The iframe/img-in-p subset is already tracked as #449

## Dependencies

- Issue #449 (iframe/img in p) should be completed first to reduce diff count

## Baseline

- DTC: 790/790 (must not regress)
- muan-blog: 2199/2218 (19 pages differ)

## Acceptance Criteria

- [ ] muan-blog site rebuilt with latest code
- [ ] DOM comparison re-run and results documented
- [ ] Every differing page categorized by root cause
- [ ] For each root cause category, either:
  - (a) Fixed in this issue, OR
  - (b) Existing sub-issue referenced (e.g., #449), OR
  - (c) New sub-issue created in `docs/tracker/`
- [ ] muan-blog match count improved from 2199/2218 (target: 2205+ after #449 lands)
- [ ] DTC DOM baseline remains at 790/790
- [ ] `cargo test` passes
- [ ] No regression on any other test site

## Test Scenarios

### Investigation: DOM diff categorization
- Build muan-blog, run DOM comparison, list all differing pages
- For each differing page, identify root cause category
- Verify heading ID differences are consistent (same algorithm issue)
- Check if datetime diffs are timezone-related or format-related
- Count how many diffs would be resolved by #449 alone

### Output verification
- After any fixes, rebuild muan-blog and re-run DOM comparison
- Verify fixed pages now match Jekyll output exactly
- Verify no previously-matching pages regressed
