# Issue 550: DTC docs -- standalone images not wrapped in paragraph tags

## Problem

In the DTC docs site (DataTalksClub/docs), 19 pages have images that are rendered as bare `<img>` tags by rustkyll, but Jekyll/kramdown wraps standalone images in `<p>` tags. This causes 26 `tag_name_differs` diffs (expected `p`, actual `img`).

Example: In `courses/course-management-platform/dashboard/index.html`:
- Jekyll: `<p><img src="..." alt="Course dashboard" width="80%" /></p>`
- Rustkyll: `<img src="..." alt="Course dashboard" width="80%" />`

The markdown source has images on their own line like:
```
![Course dashboard](/assets/images/course-management-platform/ml-dashboard.png){: width="80%"}
```

Kramdown treats a standalone image (only content in its paragraph) as still wrapped in `<p>` tags. Rustkyll is stripping the `<p>` wrapper when the paragraph contains only an image.

## Scope

Fix the kramdown renderer so that standalone images on their own line are wrapped in `<p>` tags, matching Jekyll/kramdown behavior. This is NOT about images inside other block elements -- only about paragraph-level images.

## Dependencies

None. This is an independent kramdown rendering fix.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests still passing
- [ ] New unit test: markdown with standalone image on its own line produces `<p><img ...></p>`
- [ ] New unit test: image inside a paragraph with other text still renders correctly (no double-wrapping)
- [ ] DTC docs DOM comparison improves from 38/57 to 57/57 (all 26 tag_name_differs diffs resolved)
- [ ] DTC main DOM match count does not drop below 596/790
- [ ] No other site regresses in DOM match count

## Test Scenarios

### Unit: Standalone image paragraph wrapping
- Parse markdown `![alt](url)` on its own line, verify output is `<p><img src="url" alt="alt"></p>`
- Parse markdown `![alt](url){: width="80%"}` with IAL, verify `<p><img src="url" alt="alt" width="80%"></p>`
- Parse markdown with image inside a text paragraph (`Some text ![alt](url) more text`), verify single `<p>` wrapping with text and image together
- Parse markdown with multiple images on separate lines, verify each gets its own `<p>` wrapper

### Integration: DTC docs site
- Build DTC docs site with rustkyll
- Run DOM comparison, verify 57/57 match (or at minimum, all 26 tag_name_differs resolved)
- Build DTC main site, verify no regression from 596/790

## Output Verification

- Build DTC docs: `./target/release/rustkyll build --source websites/DataTalksClub/docs --destination /tmp/dtc_docs_test`
- Check specific page: `grep '<img' /tmp/dtc_docs_test/courses/course-management-platform/dashboard/index.html` should show `<p><img ...></p>` not bare `<img>`
- Run DOM comparison: `uv run scripts/dom_compare.py --jekyll-dir websites/DataTalksClub/docs/_site_jekyll_cached --rustkyll-dir /tmp/dtc_docs_test`

## DOM Baseline

- DTC main: 596/790 (must not drop below this)
- DTC docs: 38/57 (target: 57/57)

## Log

### [PM] 2026-04-02 grooming
- Identified root cause: all 26 diffs in DTC docs are tag_name_differs expected 'p' actual 'img'
- Single pattern: standalone images on own line missing paragraph wrapper
- Affects 19 pages across course-management-platform, data-engineering-zoomcamp, and general sections
