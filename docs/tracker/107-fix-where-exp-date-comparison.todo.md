# Issue 107: Fix where_exp date comparison for course/conference pages

## Problem

Course and conference pages use where_exp with date comparisons to filter tracks/events. rustkyll produces empty results where Jekyll shows content.

Affected pages from issue #93:
- /courses/2021-winter-ml-zoomcamp.html (4.12% pixel diff)
- /conferences/2021-feb.html (2.21% pixel diff)

## Acceptance criteria

- where_exp date comparisons produce same results as Jekyll
- Course page shows syllabus sections correctly
- Conference page shows "Past days" tracks
- Both pages achieve 0% pixel diff
- All existing tests pass
