# Issue 548: beautiful-jekyll img wrapping diffs (3 remaining on sample-markdown page)

## Problem

After fixing `<details markdown="1">` rendering in issue 547, beautiful-jekyll still shows 4/5 DOM match instead of 5/5. The remaining 3 diffs on `2020-02-28-sample-markdown/index.html` are all `tag_name_differs: expected 'p', actual 'img'` -- rustkyll wraps `<img>` tags differently than Jekyll/kramdown.

## Origin

Descoped from issue 547 acceptance criteria ("beautiful-jekyll DOM improves from 4/5 to 5/5").

## Affected Sites

- beautiful-jekyll: `2020-02-28-sample-markdown/index.html` -- 3 diffs

## Acceptance Criteria

- [ ] beautiful-jekyll DOM match improves from 4/5 to 5/5
- [ ] DTC DOM match count does not drop below 596/790
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] No regressions on other sites

## Dependencies

- Issue 547 (done)
