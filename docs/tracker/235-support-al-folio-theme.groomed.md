# Issue 235: Support al-folio Jekyll theme

## Problem

al-folio is one of the most popular Jekyll themes, heavily used in academia for personal and research websites. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/alshedivat/al-folio
- **Stars:** ~11,000
- **Use case:** Academic personal websites, research portfolios
- **Notable features:** Publications via BibTeX, project cards, blog with math, Jupyter notebook integration, image galleries with lightbox, multi-language support

## Scope

1. Clone the al-folio demo site into `websites/al-folio/`.
2. Build the cloned site with both Jekyll and rustkyll.
3. Run DOM comparison against the Jekyll reference output and record the actual match rate.
4. Identify al-folio-specific rendering blockers and either fix them or create follow-up issues that reference this issue.

## Baseline

- DTC DOM baseline: 767/790

## Acceptance Criteria

- [ ] The al-folio demo site is cloned into `websites/al-folio/` and the repository state is documented in the issue log.
- [ ] Jekyll builds the demo site successfully and produces a reference `_site` output.
- [ ] rustkyll builds the demo site successfully without errors and produces HTML output for the same site.
- [ ] The DOM comparison between the Jekyll and rustkyll outputs is run and the issue log records the exact match count, differing-file count, and main diff categories.
- [ ] Representative pages that exercise al-folio features are verified in the output, including the homepage plus pages covering publications, projects, blog content with math, image galleries, and multilingual navigation if present in the demo.
- [ ] Any al-folio-specific rendering issues discovered during comparison are either fixed in this issue or explicitly tracked in follow-up issues that reference `#235`.
- [ ] The DTC DOM match count does not drop below `767/790`.

## Test Scenarios

### Integration: demo site setup
- Clone the upstream al-folio demo site into `websites/al-folio/` and verify the expected theme files and configuration are present.
- Run `jekyll build` in the demo site and confirm the reference HTML output is generated.
- Run `rustkyll build --source websites/al-folio --destination /tmp/al-folio-rustkyll` and confirm the same page set is generated.

### Integration: output comparison
- Run DOM comparison against the Jekyll reference output and record the exact match count, differing-file count, and notable diff categories.
- Inspect representative pages for publication cards, project listings, blog posts with math, image gallery pages, and multilingual pages if present in the demo.
- Verify any identified rendering blocker is either fixed or captured in a follow-up issue linked to `#235`.

## Dependencies

- None (research/benchmark task)
