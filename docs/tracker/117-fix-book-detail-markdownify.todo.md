# Issue 117: Fix book detail page markdownify pipeline

book-ml-bookcamp.html has 5% pixel diff. The newline_to_br | markdownify filter pipeline produces different output for Q&A threads with complex text.

## Acceptance criteria
- book-ml-bookcamp.html achieves 0% pixel diff
- No regressions on other book pages
