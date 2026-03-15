# Issue 103: Fix template whitespace normalization (D4, D7, D16)

Descoped from issue #90. Minor whitespace differences between Jekyll and rustkyll output:
- D4: Extra blank lines in blog posts
- D7: Minor blank line differences
- D16: Indentation differences in generated HTML

No visual impact but affects structural comparison.

Note from the user: it's okay if there extra blank lines/whitespaces in the code as long as:

- the dom tree is the same
- there's pixel-perfect match between jekyll and rustkyll

## Acceptance criteria
- Whitespace output matches Jekyll for affected pages
- No visual regressions
