# Issue 470: Inline code language-plaintext class (mlbookcamp, muan-blog)
## Problem
Inline `<code>` gets `class="language-plaintext highlighter-rouge"` but Jekyll uses just `class="highlighter-rouge"`. ~35 diffs across 10 files.
## Affected Sites
- mlbookcamp (9 files)
- muan-blog (1 file)
## Baseline
DTC 790/790. Must not regress.
