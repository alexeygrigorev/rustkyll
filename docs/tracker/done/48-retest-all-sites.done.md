# Issue 48: Retest All Cross-Site and Complex Sites

## Problem

Issues #37-44 fixed many blockers (missing filters, seo tag, include paths, highlight tag, dynamic includes, site.related_posts/pages, duplicate YAML keys, hash integer indexing). The expected results in docs/cross-site-results.md and docs/complex-site-results.md need to be verified by actually rebuilding all sites with the current codebase.

## Requirements

- Rebuild all 11 sites from docs/cross-site-results.md (alexeygrigorev + DataTalksClub repos in `websites/`)
- Rebuild all 8 sites from docs/complex-site-results.md (external Jekyll sites in `websites/`)
- Update both docs with actual verified results (page counts, static file counts, build times, status, error messages if any)
- Replace "Expected Status" / "Expected impact" sections with actual verified results
- Record any new failure modes discovered during testing

## Dependencies

- Issues #37-42 (all done)
- Issue #43 -- duplicate YAML keys (done)
- Issue #44 -- hash integer indexing (done)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors (no code changes expected, but verify build works)
- [ ] All 11 cross-site repos are rebuilt with `rustkyll build` and results recorded
- [ ] All 8 complex-site repos are rebuilt with `rustkyll build` and results recorded
- [ ] `docs/cross-site-results.md` is updated with actual results: each site row has verified Status, Pages, Static Files, Time, and Notes columns
- [ ] `docs/complex-site-results.md` is updated with actual results: each site row has verified Build Status, Pages Rendered, and Blocker columns
- [ ] The "Update" / "Expected impact" sections in both docs are replaced with verified "Actual Results" sections showing what really happened
- [ ] At least 10 of 11 cross-site repos build successfully (mlbookcamp-page expected to fail due to site-specific typo `erl_encode`)
- [ ] At least 6 of 8 complex sites build successfully (up from 1 of 8 previously)
- [ ] For any site that now builds but previously failed, the page count is recorded and is greater than 0 (confirming pages actually rendered, not just "no error")
- [ ] Any newly discovered failures are documented with error messages and root cause analysis
- [ ] No regressions: sites that previously built (7 cross-site, 1 complex) must still build successfully

## Test Scenarios

### Verification: Cross-site repos (11 sites)

For each site in `websites/alexeygrigorev/` and `websites/DataTalksClub/`:
- Run `rustkyll build` and record exit code, page count, static file count, build time
- Previously passing sites (alexeygrigorev.github.io, kids-horror-stories-ru, snippets, data-science-interviews, mlwiki.org, datatalksclub.github.io, courses) must still pass with similar page/file counts
- Previously failing sites expected to now pass: aihero (seo tag #38), little-book-of-metals-ru (normalize_whitespace #37), DataTalksClub/docs (include paths #39)
- mlbookcamp-page expected to still fail (site-specific `erl_encode` typo)

### Verification: Complex sites (8 sites)

For each site in `websites/`:
- Run `rustkyll build` and record exit code, page count, static file count, build time
- wtf-html-css: must still build (no regression)
- Jekyll Docs: expected to pass now (date_to_long_string #37)
- Edition Template: expected to pass now (seo tag #38)
- Government GitHub: expected to pass now (dynamic includes #41)
- AcademicPages: expected to pass now (include subdirectory paths #39)
- Hyde: expected to pass now (highlight #40, related_posts/pages #42)
- Open Source Guide: expected to pass now (seo #38 + hash integer indexing #44)
- Bitcoin.org: expected to pass now (duplicate YAML keys #43)

### Documentation update

- Both results docs must have a clear "Verified Results" section with the actual date of testing
- Remove or replace all "Expected" language with "Actual" / "Verified"
- Any site with a new or unexpected error gets a detailed error analysis section

## Notes

- This issue requires no code changes -- it is purely a verification and documentation update task
- The engineer should use `./scripts/cargo-safe` to avoid OOM issues on large sites
- Build times should be recorded for comparison with previous runs
- If a site fails unexpectedly, document the error but do NOT fix it in this issue -- create a new issue instead
