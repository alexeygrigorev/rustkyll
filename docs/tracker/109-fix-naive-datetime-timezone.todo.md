# Issue 109: Fix NaiveDateTime timezone handling for dates without timezone

## Problem

YAML dates without timezone (e.g. `2020-12-18 23:59:59`) are treated as NaiveDateTime by rustkyll, showing "18 Dec 2020". Jekyll treats them as UTC then converts to local time, showing "19 Dec 2020" in CET.

This causes a 51-pixel diff on /people/alexeygrigorev.html and likely affects other pages with timezone-edge dates.

## Acceptance criteria

- Dates without timezone produce same output as Jekyll
- /people/alexeygrigorev.html achieves 0 pixel diff
- No regressions on other date formatting
