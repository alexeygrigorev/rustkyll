# Issue 166: Fix blog/how-to-run-postgresql-and-pgadmin-with-docker.html (190 diffs)

Seventh highest DOM diff blog post. Investigate and fix rendering differences. TDD per pattern.

## Log

### [SWE] 2026-03-16

- Investigated: all 190 diffs were a single pattern -- rustkyll added 2-space indentation before `<li>` items in tight lists, while Jekyll/kramdown does NOT indent tight list `<li>` items
- Root cause: `indent_list_items()` in kramdown.rs line 1890 had a tight-list branch that incorrectly added `"  "` prefix to every `<li>` and `</li>` line
- Loose lists (with `<p>` inside `<li>`) correctly get indentation -- only tight lists should not be indented
- Fix: replaced the tight-list branch to pass through list content as-is (no indentation added)
- Replaced incorrect test `test_issue164_tight_list_indent` which asserted indentation should exist
- Tests added: 4 new tests
  - `test_issue166_tight_list_no_indent` -- unit test for ul tight list
  - `test_issue166_tight_list_ol_no_indent` -- unit test for ol tight list
  - `test_issue166_loose_list_still_indented` -- regression test for loose lists
  - `test_issue166_tight_list_end_to_end` -- end-to-end markdown-to-HTML test
- Build: 1551 tests pass, 0 fail
- Clippy: clean (0 warnings)
- Fmt: clean
- Site rebuild: blog/how-to-run-postgresql-and-pgadmin-with-docker.html now has 0 diffs (was 190)
- Side benefit: 48 other blog posts with tight lists also fixed
- Files modified: src/kramdown.rs
