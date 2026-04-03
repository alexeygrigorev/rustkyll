# Issue 545: Implement remaining al-folio custom Liquid tags (tabs, quote, cite)

## Problem

Issue #508 wired up `details` and `file_exists` tags but the remaining al-folio custom tags are still no-ops. Pages using these tags render without their intended HTML structure.

Descoped from #508.

## Scope

1. `{% tabs %}` / `{% endtabs %}` -- Produce `<ul>` with tab navigation and `<div>` panels matching al-folio's expected output
2. `{% quote %}` / `{% endquote %}` -- Produce styled `<blockquote>` with optional attribution
3. `{% cite %}` / `{% reference %}` / `{% bibliography %}` -- Produce reasonable stub output (e.g. `[key]`) without leaking Liquid
4. `bust_file_cache` filter -- Append cache-busting query parameter to file URLs
5. Reduce al-folio Liquid leak count toward 0

## Acceptance Criteria

- [ ] `{% tabs %}` produces `<ul>` tab navigation and `<div>` panels
- [ ] `{% quote %}` produces styled `<blockquote>` with optional attribution
- [ ] `{% cite key %}` produces `[key]` or similar stub without Liquid leak
- [ ] `bust_file_cache` filter appends query parameter
- [ ] DTC DOM match count does not regress
- [ ] `cargo build` compiles without errors; `cargo clippy` clean

## Dependencies

- Issue #508 (done)
