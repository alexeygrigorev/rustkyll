# Issue 423: Add Docker/Dockerfile syntax highlighting grammar

## Problem

Two code blocks on ml-deployment-lambda use `language-docker` but syntect
has no Dockerfile grammar. The blocks get the correct `<div>` wrapper
(fixed by #411's language recognition) but lack syntax highlighting spans
inside, causing 2 `tag_name_differs` diffs (div vs pre).

Actually, #411 was reverted because it increased total diffs. The root
issue: without a Docker grammar, adding the div wrapper exposes more
span-level diffs inside the block.

## Scope

1. Add a Dockerfile syntax definition to syntect's grammar set
2. Or: if no grammar available, investigate why div-vs-pre causes 2 diffs
   and whether there's a simpler fix

## Baseline

DTC DOM: 789/790, 3 total diffs -- neither must worsen.

## Log

### [SWE] 2026-03-28
- Investigated Jekyll cached output: Docker code blocks have full div wrapper AND
  syntax highlighting spans (FROM->k, COPY->k+s, RUN->k+shell, comments->c, etc.)
- Rustkyll had bare `<pre><code class="language-docker">` (no div, no spans)
- Root cause: "docker"/"dockerfile" not in `is_rouge_recognized_language()` and
  syntect has no Dockerfile grammar
- TDD: wrote 14 unit tests for Docker highlighting (docker_highlight.rs) and
  3 integration tests in kramdown.rs; verified tests FAIL before implementation
- Created `src/docker_highlight.rs`: Docker syntax highlighter matching Rouge output
  - FROM with/without AS, RUN with shell-like parsing, COPY/ENV/CMD/etc, comments
  - Handles builtins (install), flags (-r, --name), operators (==)
- Added "docker" and "dockerfile" to `is_rouge_recognized_language()` in kramdown.rs
- Added Docker highlight fallback in `wrap_fenced_code_blocks()` when syntect returns None
- All 17 new tests PASS
- Full suite: 3023+ tests pass, 0 failures
- Clippy: clean
- fmt: clean
- DOM: 789/790 pages, 1 total diff (was 3 total diffs -- eliminated 2 tag_name_differs)
- Remaining diff: jsonld quote style (unrelated)
- Files created: src/docker_highlight.rs
- Files modified: src/lib.rs, src/kramdown.rs
