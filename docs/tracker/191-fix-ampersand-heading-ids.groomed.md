# Issue 191: Fix ampersand handling in heading IDs

## Checklist Category

**Ampersand handling in heading IDs** -- 7 pages.

## Problem

Heading IDs generated from text containing `&` differ. Jekyll produces `free--free-to-audit-courses` (stripping the ampersand entirely) while rustkyll produces `free-amp-free-to-audit-courses` (converting `&` to `amp`).

The root cause: in HTML, `&` is encoded as `&amp;`. The `strip_html_tags` function in `src/kramdown.rs` removes HTML tags but does NOT decode HTML entities. So `&amp;` becomes literal text `&amp;`, and `slugify` strips `&` and `;` but keeps `amp`, producing `amp` in the slug.

Sample diff:
```
body > div > div > div > div > h3: attribute_differs
  expected: "id='free--free-to-audit-courses'"
  actual:   "id='free-amp-free-to-audit-courses'"
```

## Goal

Decode HTML entities (at minimum `&amp;`) in heading text before slugification, so that `&` is treated as a non-alphanumeric character and stripped by slugify.

## Affected Sites

- DataTalksClub/datatalksclub.github.io: 3 pages
- alexeygrigorev/mlwiki.org: 4 pages

## Dependencies

None.

## Approach (TDD)

1. Write a test that creates a heading `<h3>Free &amp; Free to Audit Courses</h3>`, runs `add_heading_ids`, and asserts the ID is `free--free-to-audit-courses`
2. Verify the test fails (currently produces `free-amp-free-to-audit-courses`)
3. Add HTML entity decoding in `strip_html_tags` or between `strip_html_tags` and `slugify` in `src/kramdown.rs` (around line 956)
4. Verify the test passes

## Acceptance Criteria

- [ ] Heading text containing `&amp;` produces an ID with `&` stripped (e.g., `free--free-to-audit-courses`, not `free-amp-free-to-audit-courses`)
- [ ] Other common HTML entities (`&lt;`, `&gt;`, `&quot;`, `&#39;`) are also decoded before slugification
- [ ] Numeric entities (`&#8217;`, `&#x2019;`) are decoded to their characters, which are then stripped by slugify if non-alphanumeric
- [ ] Existing heading ID tests still pass
- [ ] `cargo test` passes

## Test Scenarios

### Unit: Ampersand in heading ID (write FIRST, must fail before fix)

- **Test `test_heading_id_ampersand_stripped`**: HTML input `<h3>Free &amp; Free to Audit Courses</h3>`. Assert ID is `free--free-to-audit-courses`.
- **Test `test_heading_id_lt_gt_stripped`**: HTML input `<h3>A &lt; B &gt; C</h3>`. Assert ID is `a--b--c` (entities decoded, then non-alphanumeric stripped).
- **Test `test_heading_id_numeric_entity`**: HTML input `<h3>It&#8217;s a Test</h3>`. Assert the right-single-quote is stripped, producing `its-a-test`.
- **Test `test_heading_id_no_entities_unchanged`**: HTML input `<h3>Simple Heading</h3>`. Assert ID is `simple-heading` (no change from current behavior).

### Regression: Existing heading IDs preserved

- **Test `test_heading_id_cyrillic_unchanged`**: Cyrillic headings should still work correctly.
- **Test `test_heading_id_numeric_prefix_preserved`**: Headings starting with numbers should still preserve the number prefix.

### Integration: Output verification

- Build DTC site and inspect `free-data-engineering-courses.html` to verify the heading ID for "Free & Free to Audit Courses".
