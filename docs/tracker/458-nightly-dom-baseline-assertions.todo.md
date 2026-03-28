# Issue 458: Store DOM baselines and assert no regression in nightly CI

## Problem

We need a checked-in baseline file so the nightly CI can detect when
any site's DOM match count drops. Currently there's no single source
of truth for "what the expected match count is" per site.

## Scope

1. Create `docs/dom-baselines.json` with current match counts for all sites:
   ```json
   {
     "DataTalksClub/datatalksclub.github.io": 790,
     "large-blog-3000": 3001,
     "large-docs-site": 801,
     "muan-blog": 2195,
     "DataTalksClub/docs": 48,
     "lanyon": 6,
     "beautiful-jekyll": 4,
     ...
   }
   ```

2. The nightly CI job reads this file and asserts each site's match
   count >= the baseline value

3. When we fix a site and improve its count, we update the baseline
   in the same commit — so the new higher count becomes the floor

4. Any PR that drops a site below its baseline fails CI

## Rules

- Baselines only go UP, never down
- Every rendering commit should update the baseline if it improved a site
- The nightly job fails loudly on ANY regression, not just DTC
