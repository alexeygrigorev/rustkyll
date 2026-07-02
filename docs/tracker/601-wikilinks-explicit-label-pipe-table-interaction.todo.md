# Issue 601: wikilinks `[[target|label]]` mangled by GFM table parsing

Follow-up to #600 (extension framework + wikilinks). Do NOT reopen #600 for this;
it is a pre-existing markdown-renderer limitation surfaced during #600 QA.

## Problem

rustkyll's markdown renderer turns **any** single line containing a `|` into a
GFM-style table *before* the post-render HTML transforms (including wikilinks)
run. As a result, an author writing the explicit-label wikilink form inline in a
markdown paragraph:

```markdown
See [[event-tracking|event tracking]] for details.
```

gets the `|` consumed by table parsing first, so the wikilinks transform later
sees mangled HTML (e.g. `[[event-tracking</td><td>event tracking]]`) and
correctly leaves it alone — the link is never resolved.

QA confirmed this reproduces with the wikilinks extension **fully disabled**
(`This is a sentence with a | pipe.` -> `<table>...</table>`), so it is
independent of #600. The primary `[[target]]` form (no pipe) works end-to-end;
only the explicit-label form is affected in real `.md` files. The wikilinks
transform itself handles `[[target|label]]` correctly given proper HTML input
(unit tests `test_explicit_label_preserved`, `test_broken_explicit_label_preserved`
pass).

This blocks the podwiki explicit-label authoring workflow, which is a motivating
use case for #600.

## Scope

Make the explicit-label `[[target|label]]` form work end-to-end in real markdown
files. Investigate options, e.g.:

- Detect and exempt single-line `[[...|...]]` spans from GFM table detection in
  the markdown pipeline (only lines that are *actually* pipe-tables should
  become tables — a lone inline `|` should not).
- Or resolve wikilinks pre-render (before markdown), so `[[...]]` never reaches
  the table parser. (Weigh against the current post-render design and the
  code/pre skip guarantees.)
- Or provide an escaping / alternative delimiter path.

Pick the approach with the least regression risk to the DTC DOM baseline. The
broader "lone `|` in a paragraph becomes a table" behavior may be a legitimate
kramdown-vs-GFM difference worth fixing on its own; scope the fix carefully.

## Acceptance Criteria

- [ ] A markdown paragraph containing `[[target|label]]` renders to a resolved
      anchor with the explicit label (end-to-end via the release binary on a
      fixture), not a `<table>` fragment.
- [ ] The primary `[[target]]` form still works end-to-end (no regression).
- [ ] `[[...]]` inside `<code>`/`<pre>` still left untouched.
- [ ] Real markdown pipe-tables (proper header + `|---|` separator rows) still
      render as tables (no regression to legitimate table support).
- [ ] DTC DOM match count must not drop below the #600 baseline of **788/790**.
      Verify with `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
      once the DTC source is present.
- [ ] `./scripts/cargo-safe test` green, clippy clean, `cargo fmt --check` clean.
- [ ] TDD: test-first, log the fail-then-pass cycle.

## Test Scenarios

- Markdown fixture: `See [[event-tracking|event tracking]] here.` ->
  resolved anchor with label `event tracking`, no `<table>`.
- Markdown fixture: a genuine pipe-table (with `|---|` separator) still becomes
  a `<table>`.
- Markdown fixture: a lone `|` in prose (`cost is $5 | $10`) — define and test
  the intended behavior.
- `[[target]]` (no pipe) unaffected.

## Dependencies

- #600 (extension framework + wikilinks) must be `.done.md` first.

## Notes

- Surfaced by #600 QA (2026-07-02). See #600 `## Log` [QA] entry for the exact
  reproduction.
