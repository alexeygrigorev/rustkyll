# Issue 122: Fix data listing order on DTC tools and course pages

## Priority

HIGH — tools.html (1.27%) and course-ml-zoomcamp.html (4.11%) pixel diff due to listing order.

## Problem

Events and tools appear in different order than Jekyll. This affects:
- /tools.html — tool listing order differs
- /courses/2021-winter-ml-zoomcamp.html — event listing order differs

## Root cause

Data file iteration order or collection sorting differs from Jekyll. Jekyll likely uses insertion order from YAML data files or specific sort criteria.

## Acceptance criteria

- tools.html achieves 0% pixel diff
- course-ml-zoomcamp.html achieves 0% pixel diff
- Data listing order matches Jekyll exactly
- No regressions on other pages
