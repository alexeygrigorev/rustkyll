# Issue 162: Fix blog/how-to-setup-lightweight-local-version-for-airflow.html (403 diffs)

Third highest DOM diff blog post. Investigate and fix rendering differences. TDD per pattern.

## Log

### [SWE] 2026-03-16

#### Analysis

Built both Jekyll and rustkyll, ran dom_compare on the airflow blog post.

**Result: 0 DOM differences.** All 403 diffs have been resolved by the fixes from issues 158 and 160 (which are still uncommitted). The blog post HTML is now byte-identical between Jekyll and rustkyll.

The original 403 diffs were caused by these patterns (all fixed in prior issues):
1. **figcaption `<p>` stripping** -- Fixed in issue 158: `<figcaption><p>...</p></figcaption>` is now preserved
2. **Code block `</pre></div></div>` splitting** -- Fixed in issue 158: `add_block_spacing` no longer breaks code wrapper closing tags
3. **Blank line after `</figure>`** -- Fixed in issue 158: `</figure>` added to block spacing tags
4. **List indentation** -- Fixed in issue 158: loose list items indented with 2/4 spaces
5. **Python syntax highlighting** -- Fixed in issues 158/160: dict colon, module dot splitting, print/input classification
6. **SQL syntax highlighting** -- Fixed in issue 160: SQL post-processing wraps bare tokens

#### Tests added (TDD)

6 new tests for issue 162 verifying the patterns from the airflow blog post are handled correctly:

- `test_issue162_figcaption_p_preserved_through_pipeline` -- full `markdown_to_html` pipeline preserves `<p>` inside `<figcaption>` within `<figure>`
- `test_issue162_figcaption_p_with_links_preserved_through_pipeline` -- figcaption with `<a>` links inside `<p>` preserved through pipeline
- `test_issue162_figcaption_p_preceded_by_markdown` -- `<figure>` block preceded by markdown text
- `test_issue162_figure_with_figcaption_p_links_preserved` -- `strip_paragraphs_in_html_blocks` preserves `<p>` inside `<figcaption>` within `<figure>` (exact airflow blog pattern)
- `test_issue162_figure_figcaption_postprocess` -- end-to-end `postprocess` for airflow figcaption pattern
- `test_issue162_normalize_figcaption_whitespace_with_p` -- figcaption whitespace normalization works with `<p>` content

#### Results
- DOM differences: 403 -> 0 (100% reduction)
- Tests: 1319 lib + integration tests pass, 0 failures
- Clippy: clean (0 warnings with -D warnings)
- Fmt: clean

#### Files modified
- `src/frontmatter.rs` -- 3 new tests for issue 162
- `src/kramdown.rs` -- 3 new tests for issue 162
- `docs/tracker/162-fix-blog-airflow.in-progress.md` -- this log
