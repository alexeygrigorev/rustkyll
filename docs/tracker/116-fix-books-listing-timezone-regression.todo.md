# Issue 116: Fix books listing timezone regression

books.html regressed to 0.38% pixel diff (27044 pixels) after timezone fix. The system timezone conversion changed book dates causing visual differences.

## Acceptance criteria
- books.html achieves 0% pixel diff
- No regressions on other pages
