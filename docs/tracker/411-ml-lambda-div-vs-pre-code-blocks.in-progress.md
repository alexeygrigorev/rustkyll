# Issue 411: ml-lambda page has `<div>` vs `<pre>` for code blocks

## Problem

On the ml-deployment-lambda page, some code blocks render as `<div>`
in Jekyll but `<pre>` in rustkyll (or vice versa):

```
tag_name_differs - expected: 'div', actual: 'pre'
```

This affects 2 code blocks (child[46] and child[56]) -- both are
fenced code blocks with language `docker`.

## Root Cause

Jekyll/Rouge recognizes `docker` as a valid language and wraps code
blocks in `<div class="language-docker highlighter-rouge"><div class="highlight"><pre class="highlight"><code>`.
Rustkyll's `is_rouge_recognized_language()` list did not include
`docker` or `dockerfile`, so these blocks fell through to the bare
`<pre><code class="language-docker">` path.

## Acceptance Criteria

1. `docker` and `dockerfile` fenced code blocks get the
   `<div class="language-X highlighter-rouge">` wrapper div
2. DTC DOM: 788/790 pages (must not drop)
3. DTC DOM: 228 total diffs (must not increase)
4. All existing tests pass
5. Clippy clean, fmt clean

## Baseline

DTC DOM: 788/790 pages, 228 total diffs

## Log

### [SWE] 2026-03-28
- Investigated: Jekyll output has `<div class="language-docker highlighter-rouge">` wrapping
  for `docker` code blocks; rustkyll produced bare `<pre><code class="language-docker">`
- Root cause: `is_rouge_recognized_language()` in kramdown.rs missing "docker"/"dockerfile"
- TDD: wrote 2 failing tests (test_fenced_code_wrapping_docker_language,
  test_fenced_code_wrapping_dockerfile_language)
- Tests FAILED as expected: got bare `<pre><code>` instead of div wrapper
- Fix: added "docker" and "dockerfile" to `is_rouge_recognized_language()`
- Tests PASS after fix
- Full suite: all pass, 0 failures
- Clippy: clean (no warnings)
- fmt: clean
- DOM: 788/790 pages, 228 total diffs (unchanged from baseline)
- `tag_name_differs` entries eliminated from ml-deployment-lambda diffs
- Files modified: src/kramdown.rs
