# Issue 231: DTC FAQ JSON-LD integration test

## Problem

Issue 218 fixed FAQ `acceptedAnswer.text` whitespace diffs by adding `add_block_spacing` and `indent_list_items` to `postprocess_for_filter()`. The fix is covered by 6 unit tests, but test scenario #7 from the spec (an ignored integration test that builds the full DTC site and compares all FAQ JSON-LD blocks character-for-character) was not implemented.

## Origin

Descoped from issue 218 during acceptance review.

## Scope

Write an `#[ignore]` integration test that:

1. Builds the DTC site with Rustkyll
2. Extracts all `acceptedAnswer.text` values from FAQ JSON-LD blocks in both `_site_jekyll/` and `_site/`
3. Asserts they are character-for-character identical for all 9 FAQ pages
4. Prints which pages/questions differ on failure

## Affected pages (9 total)

Matching (should stay matching):
- `blog/guide-to-free-online-courses-data-science-ml.html`
- `blog/open-source-free-ai-agent-evaluation-tools.html`
- `blog/free-machine-learning-courses.html`
- `blog/slack-communities.html`

Fixed by issue 218 (should now match):
- `blog/ai-dev-tools-zoomcamp-2025-...`
- `blog/llm-zoomcamp.html`
- `blog/mlops-zoomcamp.html`
- `blog/data-engineering-zoomcamp.html`
- `blog/machine-learning-zoomcamp.html`

## Dependencies

- Issue 218 (done)

## Acceptance Criteria

- [ ] An `#[ignore]` integration test exists that compares all FAQ `acceptedAnswer.text` values between Jekyll and Rustkyll output for the DTC site
- [ ] The test passes when run with `cargo test --ignored`
- [ ] The test prints clear diagnostics on failure (page URL, question index, expected vs actual)

## Log

- 2026-03-18: Created as follow-up from issue 218 acceptance review (descoped test scenario #7).
