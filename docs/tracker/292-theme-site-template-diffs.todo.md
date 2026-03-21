# Issue 292: Fix remaining theme site template/layout DOM diffs

## Problem

After issue 290 eliminated all syntax highlighting diffs across theme sites, 7 of 10 theme sites still do not reach a perfect 2/2 DOM match. The remaining diffs are all template/layout issues, not syntax highlighting.

## Specific Diffs

### href attribute diffs (7 sites)

The following sites have an empty `href` in the Jekyll cached output but rustkyll produces the actual GitHub URL:

- dinky-theme (2 diffs): `body > div > header > ul > li > a` href
- hacker-theme (2 diffs): `body > header > div > section > a` href
- leap-day-theme (2 diffs): similar href attribute diff
- merlot-theme (2 diffs): similar href attribute diff
- midnight-theme (2 diffs): similar href attribute diff
- time-machine-theme (4 diffs): two different `<a>` elements with href diffs

These likely stem from a `site.github` variable or similar template logic that produces an empty string in the cached Jekyll output but a real URL in rustkyll.

### Text/element diffs (1 site)

- primer-theme (6 diffs): "This site is open source." text differs, missing `<a>` element -- likely a conditional link that Jekyll renders differently when `site.github.repository_url` is empty.

## Acceptance Criteria

- [ ] All 10 theme sites reach 2/2 DOM match (0 total differences each)
- [ ] No regressions on other sites
- [ ] `cargo test` passes

## Dependencies

- Issue 290 (done)
