# Issue 231: DTC JSON-LD integration test

## Problem

The DTC main site has 604 `jsonld_value_differs` diffs in the DOM comparison report. These are the single largest diff category, accounting for a significant portion of the 248 non-matching pages (539/787 = 68% match rate). No integration test currently validates JSON-LD output against the Jekyll reference.

### Root cause analysis (from `docs/comparison/dom-details/DataTalksClub-datatalksclub.github.io.txt`)

The 604 diffs break down into 4 distinct root causes:

| Category | Count | Field | Root Cause |
|----------|-------|-------|------------|
| Date formatting (startDate) | 193 | `@graph[1].startDate` | Rustkyll renders dates as `2025-11-07`, Jekyll renders `2025-11-07 00:00:00 +0100` (full datetime with local timezone). The `date` field on podcast episodes with explicit dates gets expanded to `YYYY-MM-DD 00:00:00 +0000` by `normalize_frontmatter_date`, but Jekyll uses the configured timezone (e.g., `+0100` for CET), not UTC. |
| Date formatting (endDate) | 193 | `@graph[1].endDate` | Build-timestamp noise. Both Jekyll and Rustkyll use `site.time` for episodes without dates, but the timestamps differ because the builds happened at different times and in different timezones. These 193 diffs are **expected** and should be excluded from comparison. |
| Smart quotes in bios | 156+2 | `@graph[0].about[N].description` | Jekyll's Kramdown converts ASCII apostrophes/quotes to Unicode smart quotes (U+2019, U+201C, U+201D) in people bios. Rustkyll does not. Already tracked in issues #211 and #247. |
| Transcript content | 59 | `@graph[2].transcript` | Various differences in transcript text rendering (timestamp formatting like `[0.0]` vs `[0:00]`, smart quotes in transcript text, trailing whitespace). |
| Description truncation | 1 | `description` | Different truncation behavior: `detecti...` (3 dots) vs `detectio..` (2 dots). |

### Impact by fixability

- **Fixable now (193 diffs):** startDate timezone formatting -- requires making date rendering timezone-aware
- **Expected noise (193 diffs):** endDate build-timestamp -- must be excluded from any comparison test
- **Tracked elsewhere (158 diffs):** smart quote conversion -- issues #211, #247
- **Low priority (60 diffs):** transcript content and description truncation -- mixed causes

## Origin

Descoped from issue 218 during acceptance review. Expanded during grooming to cover all JSON-LD diff categories, not just FAQ.

## Scope

Write an `#[ignore]` integration test suite that:

1. Builds the DTC site with Rustkyll (or uses pre-built `_site/` output)
2. Reads the corresponding Jekyll output from `_site_jekyll/`
3. Extracts and compares all JSON-LD blocks between the two, field by field
4. Categorizes each diff by root cause (date formatting, smart quotes, transcript, other)
5. Reports a structured summary showing which categories still have diffs

Additionally, write a targeted regression test for the FAQ JSON-LD fix from issue 218.

### What this issue does NOT include

- Actually fixing the date formatting (startDate timezone) -- that should be a separate issue
- Fixing smart quotes -- tracked in issues #211, #247
- Fixing transcript diffs -- separate issue if needed

## Affected pages

### FAQ pages (9 total, should all match after issue 218)
- `blog/guide-to-free-online-courses-data-science-ml.html`
- `blog/open-source-free-ai-agent-evaluation-tools.html`
- `blog/free-machine-learning-courses.html`
- `blog/slack-communities.html`
- `blog/ai-dev-tools-zoomcamp-2025-free-course-to-master-coding-assistants-agents-and-automation.html`
- `blog/llm-zoomcamp.html`
- `blog/mlops-zoomcamp.html`
- `blog/data-engineering-zoomcamp.html`
- `blog/machine-learning-zoomcamp.html`

### Podcast pages (~193 pages with date diffs, ~156 with bio description diffs, ~59 with transcript diffs)

All pages under `podcast/` that have JSON-LD structured data.

### People pages (1 page with description truncation diff)

`people/grainnemcknight.html`

## Dependencies

- Issue 218 (done) -- FAQ JSON-LD fix

## Acceptance Criteria

- [ ] An `#[ignore]` integration test `test_faq_jsonld_matches_jekyll` exists that compares all FAQ `acceptedAnswer.text` values between Jekyll and Rustkyll output for the 9 FAQ pages listed above
- [ ] The FAQ test passes when run with `cargo test -- --ignored test_faq_jsonld`
- [ ] The FAQ test prints clear diagnostics on failure (page path, question index, expected vs actual with diff highlighting)
- [ ] An `#[ignore]` integration test `test_podcast_jsonld_diff_summary` exists that extracts all JSON-LD from podcast pages and categorizes diffs by root cause (date_format, smart_quote, transcript, other)
- [ ] The podcast summary test prints a structured report: count of diffs per category, list of affected pages per category, and sample expected/actual values
- [ ] The podcast summary test explicitly excludes `endDate` diffs where both values look like build timestamps (i.e., both are recent datetimes within hours of each other) since these are expected noise
- [ ] The podcast summary test documents the current state (expected to have diffs in date_format, smart_quote, and transcript categories) -- it should NOT assert zero diffs, but should assert that diff counts are within expected ranges so regressions are caught
- [ ] All tests compile and run without panics against the DTC site
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes

## Test Scenarios

### Integration: FAQ JSON-LD regression test (`#[ignore]`)
- Load pre-built Jekyll output from `_site_jekyll/` and Rustkyll output from `_site/` for all 9 FAQ pages
- Extract `<script type="application/ld+json">` blocks from each page
- Parse as JSON, navigate to `mainEntity[N].acceptedAnswer.text` for each FAQ question
- Assert character-for-character match between Jekyll and Rustkyll for every question on every page
- On failure: print page path, question index, and a clear diff showing where the strings diverge

### Integration: Podcast JSON-LD diff summary (`#[ignore]`)
- Load pre-built Jekyll and Rustkyll output for all podcast pages
- Extract and parse all JSON-LD blocks
- Compare field-by-field: `@graph[1].startDate`, `@graph[1].endDate`, `@graph[0].about[N].description`, `@graph[2].transcript`, and all other fields
- Classify each diff into categories:
  - `date_format`: startDate values that differ only in time/timezone formatting
  - `build_timestamp`: endDate values that are both recent datetimes (exclude from failure counts)
  - `smart_quote`: values that differ only in ASCII vs Unicode quote characters
  - `transcript`: diffs in the transcript field
  - `other`: anything else
- Print summary table with counts per category
- Assert: FAQ diffs = 0 (regression guard), date_format diffs <= 200, smart_quote diffs <= 160, transcript diffs <= 65
- These upper bounds should be the current state + small margin; if a fix reduces diffs, the bounds should be tightened

### Unit: JSON-LD extraction helper
- Test the `extract_jsonld_blocks` function (already exists in `tests/integration_jsonld.rs`) with HTML containing 0, 1, and multiple JSON-LD blocks
- Test classification of diff types (date_format vs smart_quote vs transcript) with sample values

## Log

- 2026-03-18: Created as follow-up from issue 218 acceptance review (descoped test scenario #7).
- 2026-03-19: [PM] Groomed. Expanded scope from FAQ-only to comprehensive JSON-LD diff analysis based on investigation of `docs/comparison/dom-details/DataTalksClub-datatalksclub.github.io.txt`. Identified 4 root causes across 604 diffs. Kept issue focused on testing/reporting (not fixing) with regression guards.

### [SWE] 2026-03-19
- Created `tests/integration_jsonld_comparison.rs` with all required tests
- TDD cycle for unit tests:
  - Wrote 15 unit tests covering: extract_jsonld_blocks (0, 1, multiple, unicode), classify_diff (date_format, build_timestamp, smart_quote, transcript, other), compare_json_values (identical, different, nested), is_smart_quote_diff (true/false), looks_like_date
  - Ran tests: all 15 PASS
- Wrote `#[ignore]` integration test `test_faq_jsonld_matches_jekyll`: compares all 9 FAQ pages' acceptedAnswer.text values between Jekyll and Rustkyll output, with clear diagnostics (page path, question index, divergence point)
- Wrote `#[ignore]` integration test `test_podcast_jsonld_diff_summary`: extracts all JSON-LD from podcast pages, categorizes diffs (date_format, build_timestamp, smart_quote, transcript, other), prints structured summary, excludes endDate build-timestamp noise, asserts regression bounds (date_format<=200, smart_quote<=160, transcript<=65, FAQ diffs=0)
- Build: 15 unit tests pass, 2 ignored integration tests, clippy clean, fmt clean
- Full project test suite: all tests pass
- Files created: tests/integration_jsonld_comparison.rs
- Files modified: docs/tracker/231-dtc-faq-jsonld-integration-test.in-progress.md

### [QA] 2026-03-19
- Found truncate_str byte-boundary panic on multi-byte UTF-8 -- fixed by changing to char_indices

### [PM] 2026-03-19 -- ACCEPTED
- All 11 acceptance criteria met
- 15 unit tests pass, 2 #[ignore] integration tests correctly gated
- Tests are meaningful: cover extraction (0/1/multiple/unicode), classification (all 5 diff categories), JSON comparison (identical/different/nested), smart quote detection, date detection
- UTF-8 truncation bug found by QA is properly fixed using char_indices
- Clippy clean, fmt clean
- No descoped items
