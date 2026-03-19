# Issue 250: Fix DTC/docs layout template `<hr />` whitespace causing DOM diffs

## Problem

After issue 246 fixed IAL parsing, children nav, JSON-LD encoding, and language-plaintext, 9 of the remaining 11 DTC/docs DOM differences are caused by whitespace around `<hr />` in rustkyll's layout template output.

Jekyll produces `<hr />` inline (no surrounding whitespace), while rustkyll produces it with whitespace that causes BeautifulSoup to construct a different DOM tree (specifically `<hr /> > <footer>` nesting). This affects all pages that have children nav sections because the unclosed `<li>` elements change parser state, making the whitespace difference visible.

The affected 9 pages all show a `<hr /> > <footer>` DOM parser artifact.

## Scope

- Ensure `<hr />` in rustkyll's rendered layout output matches Jekyll's whitespace behavior
- This should push DTC/docs from 46/57 to 55/57 (the remaining 2 are smart quotes and emphasis parsing, tracked by issues 211 and 247)

## Dependencies

- Issue 246 (in-progress/done) -- provides the baseline of 46/57

## Acceptance Criteria

- [ ] `<hr />` elements in layout template output have no extra whitespace compared to Jekyll
- [ ] DTC/docs DOM comparison shows at least 53/57 matches
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` passes

## Origin

Descoped from issue 246 AC12 (target was 50/57, achieved 46/57 due to this whitespace issue).
