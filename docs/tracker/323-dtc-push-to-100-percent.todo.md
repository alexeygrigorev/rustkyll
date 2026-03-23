# Issue 323: Push DTC DOM coverage to 100% (745/790 → 790/790)

## Problem

DTC matches 745/790 (94%). 45 pages have remaining diffs. Goal: match Jekyll output exactly on all 790 pages.

## Remaining diff categories (45 pages)

### Pages with 1-3 diffs (~10 pages)
- Minor text/attribute differences
- Fixable individually

### Book review comments (~20 pages)
- `newline_to_br | markdownify` pipeline produces different list structure
- Nested lists break out of `<li>`
- `<br>` placement differs

### Syntax highlighting (~8 pages)
- YAML/Python/Bash token class mismatches
- Code block structure differences

### Markdown edge cases (~5 pages)
- Escaped underscores, zero-width spaces in URLs
- Emphasis + link + IAL interactions

### Structural / complex (~2 pages)
- Missing `<script>` elements (lambda page)
- Duplicate slug resolution (data-professionals page)

## Acceptance Criteria

- [ ] DTC DOM match reaches 790/790 (100%)
- [ ] No regressions on other sites
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
