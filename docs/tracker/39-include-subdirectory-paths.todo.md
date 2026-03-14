# Issue 39: Support Include Paths with Subdirectory Separators

## Problem

Cross-site testing (Issue 32) revealed that `{% include %}` tags with path separators (`/`) in the filename are not parsed correctly by the Liquid template engine.

Examples:
- `{% include icons/icons.html %}` -- fails to parse
- `{% include course-structured-data/data-engineering-zoomcamp-structured-data.html %}` -- fails to parse

The Liquid parser treats the `/` as an unexpected character instead of part of the filename.

## Found In

- `DataTalksClub/docs` -- uses `{% include icons/icons.html %}`
- `DataTalksClub/datatalksclub.github.io` -- uses `{% include course-structured-data/*.html %}` (6 posts affected, currently produce warnings)

## Requirements

- Update the include tag parsing to allow `/` in include file paths
- Resolve include paths relative to the `_includes/` directory
- Support nested subdirectory structures within `_includes/`

## Dependencies

- None
