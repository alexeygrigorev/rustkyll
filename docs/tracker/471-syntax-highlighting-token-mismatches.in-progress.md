# Issue 471: Syntax highlighting token class mismatches (mlwiki, lanyon)
## Problem
Token classes differ from Jekyll/Rouge: `k` vs `o`, `nt` vs `na`, etc. ~25+ diffs.
## Affected Sites
- mlwiki (multiple files)
- lanyon (1 file — highlight linenos)
## Baseline
DTC 790/790. Must not regress.
