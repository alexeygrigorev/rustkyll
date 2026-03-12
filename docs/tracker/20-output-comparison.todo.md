# Issue 20: Output Comparison with Jekyll

## Description

Compare rustkyl output against the original Jekyll build to find and fix discrepancies. Build both versions and diff the HTML output.

## Dependencies

- Issue 19 (CLI and full build)

## Scope

- Build the site with both Jekyll and rustkyl
- Compare key pages: homepage, 5 people, 5 posts, 5 books, 5 podcast episodes
- Document and fix any content differences
- Acceptable differences: whitespace, attribute ordering, minor formatting
- Unacceptable differences: missing content, broken links, wrong URLs, missing metadata
- Create a comparison test that can be run to catch regressions
