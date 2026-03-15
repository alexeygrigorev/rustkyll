# Issue 114: Fix kramdown bare text auto-wrapping between blocks

Affects courses/2021-winter-ml-zoomcamp.html (4.12% pixel diff). Kramdown auto-wraps loose inline text between block elements (h3, ul) in <p> tags. Pulldown-cmark does not.

## Acceptance criteria
- Bare text between block elements wrapped in <p> matching kramdown
- Course page achieves 0% pixel diff
