# Issue 171: Fix layout not applied when Liquid conditionals in layout preamble

## Problem

Many sites have layouts that begin with Liquid logic (assign, if/elsif/else) before the `<!doctype>` or `<html>` tag. When rustkyll fails to evaluate these Liquid blocks, the layout is silently not applied, and pages are rendered as bare HTML fragments (just the markdown-to-HTML content, no `<head>`, `<body>`, or layout wrapper).

This is the single largest source of DOM diffs across the benchmark: **over 4,400 files** across multiple sites render without layouts.

## Root cause

The `default.html` layout in muan-blog starts with:
```liquid
{% if page.path contains "zh-TW" %}
  {% assign lang = "zh-TW" %}
{% elsif page.path contains "de-DE" %}
  {% assign lang = "de-DE" %}
{% else %}
  {% assign lang = "en-US" %}
{% endif %}
<!doctype html>
<html lang="{{ lang }}">
```

When the Liquid `if/elsif/else` or `assign` evaluation fails or is skipped, the entire layout is not applied. The rustkyll output is just the bare content (e.g., `<p>...text...</p>`) without any wrapping HTML structure.

## Affected sites (from DOM analysis)

| Site | Files affected | Pattern |
|------|---------------|---------|
| muan-blog | ~2187/2218 (99%) | Layout starts with `{% if page.path contains %}` |
| opensource-guide | ~336/388 (87%) | i18n pages, layout with conditional logic |
| just-the-docs | 47/47 (100%) | just-the-docs theme layout |
| DataTalksClub/docs | 57/57 (100%) | just-the-docs theme layout |
| documentation-theme-jekyll | ~97/98 (99%) | Complex data-driven layout |
| alexeygrigorev/snippets | ~17/25 (68%) | Layout with conditionals |
| government-github | ~19/21 (90%) | Layout with conditionals |

## Acceptance criteria

- [ ] Layouts that start with Liquid logic (assign, if/elsif/else, for) before HTML doctype are correctly processed
- [ ] muan-blog pages render with full `<html>`, `<head>`, `<body>` wrapping (spot-check 5 pages)
- [ ] opensource-guide i18n pages (ar/, zh-TW/, etc.) render with layout applied
- [ ] just-the-docs pages render with layout applied
- [ ] Existing tests continue to pass

## Dependencies

None -- this is a core Liquid/layout rendering bug.

## Log

### [SWE] 2026-03-17

**Root cause analysis:**

Two issues caused layouts with Liquid conditionals to fail:

1. **Missing `page.path` for collection items**: The generator injected `page.path` for standalone pages (`generate_pages`) but NOT for collection items (`generate_collection`). When layouts used `{% if page.path contains "zh-TW" %}`, `page.path` was nil, causing the liquid crate's `contains` operator to error with "Expected string | array | object, found `nil`".

2. **Liquid crate's strict `contains` on nil**: Jekyll treats `nil contains "x"` as `false`. The liquid crate (v0.26) raises a runtime error. When layout rendering fails, the generator falls back to writing bare HTML content without the layout wrapper.

**Fixes applied:**

1. **`src/generator.rs`**: Added `page.path` injection for collection items (using `item.source_path`), matching what was already done for standalone pages.

2. **`src/template/engine.rs`**: Added `preprocess_nil_contains()` function that rewrites `{% if EXPR contains "STR" %}` to `{% if EXPR and EXPR contains "STR" %}`. The `and` operator short-circuits on nil (falsy), preventing the `contains` check from ever evaluating on nil values. This matches Jekyll's lenient behavior.

   Helper functions: `rewrite_contains_with_nil_guard()`, `rewrite_contains_in_expr()`, `extract_last_operand()`.

**Tests added (12 total):**

In `src/template/layout.rs`:
- `test_issue171_layout_with_conditionals_before_doctype` - Layout with if/elsif/else before doctype renders correctly
- `test_issue171_layout_with_assign_before_doctype` - Layout with assign before doctype renders correctly
- `test_issue171_contains_with_nil_value` - Nil page.path falls through to else branch
- `test_issue171_contains_nil_in_elsif` - Nil variable in elsif also handled

In `src/template/engine.rs`:
- `test_issue171_preprocess_nil_contains_simple_if` - Preprocessing adds nil guard to simple if
- `test_issue171_preprocess_nil_contains_elsif` - Preprocessing adds nil guard to elsif
- `test_issue171_preprocess_nil_contains_no_change_for_other_tags` - Non-if tags unchanged
- `test_issue171_preprocess_nil_contains_no_change_without_contains` - If without contains unchanged
- `test_issue171_preprocess_nil_contains_with_or` - Multiple contains with or handled
- `test_issue171_nil_contains_render_with_nil_variable` - End-to-end nil variable render
- `test_issue171_nil_contains_render_with_set_variable` - End-to-end set variable render
- `test_issue171_preprocess_preserves_dash_whitespace_control` - Whitespace control dashes preserved

**Build results:** 1594 tests pass, 0 fail, clippy clean, fmt clean

**Files modified:**
- `src/generator.rs` - Added page.path injection for collection items
- `src/template/engine.rs` - Added preprocess_nil_contains and helper functions, 8 tests
- `src/template/layout.rs` - Added 4 tests
