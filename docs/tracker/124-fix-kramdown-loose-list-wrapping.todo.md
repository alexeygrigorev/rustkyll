# Issue 124: Fix kramdown loose list <p> wrapping

## Problem

mojombo-blog posts show pixel diffs (3.49%, 1.56%) from kramdown's loose list <p> wrapping. When a list item contains multiple paragraphs (separated by blank lines), kramdown wraps each paragraph in <p> tags. Pulldown-cmark may handle this differently.

Related to issue #114 (bare text wrapping) but specific to list items with multiple paragraphs.

## Acceptance criteria
- mojombo-blog post-readme-driven achieves 0% pixel diff
- mojombo-blog post-open-source achieves 0% pixel diff
- List items with blank-line-separated paragraphs match kramdown output
- No regressions
