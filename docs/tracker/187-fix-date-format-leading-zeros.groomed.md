# Issue 187: Fix date formatting missing leading zeros

## Checklist Category

**Date formatting missing leading zeros** -- 673 pages (muan-blog).

## Problem

Dates formatted in Liquid templates lose leading zeros. Jekyll outputs `2023/07/11 15:27` but rustkyll outputs `2023/7/11 15:27`. This affects the `date` Liquid filter when format strings use `%m`, `%d`, `%H`, etc.

Sample diff (muan-blog):
```
body > header > div > h1: text_differs
  expected: 'Story, 2023/07/11 15:27'
  actual:   'Story, 2023/7/11 15:27'
```

## Goal

Ensure the `date` Liquid filter pads month (`%m`), day (`%d`), hour (`%H`), minute (`%M`), and second (`%S`) with leading zeros, matching Ruby/Jekyll strftime behavior.

## Affected Sites

- muan-blog: 673 pages affected

## Dependencies

None.

## Approach (TDD)

1. Write a test that formats date `2023-07-05` with format `%Y/%m/%d` and asserts output is `2023/07/05` (not `2023/7/5`)
2. Verify the test fails
3. Fix the date filter in `src/template/filters/date.rs` to properly pad values
4. Verify the test passes

## Acceptance Criteria

- [ ] `{{ date | date: "%m" }}` for July outputs `07`, not `7`
- [ ] `{{ date | date: "%d" }}` for the 5th outputs `05`, not `5`
- [ ] `{{ date | date: "%H" }}` for 3 AM outputs `03`, not `3`
- [ ] `{{ date | date: "%Y/%m/%d %H:%M" }}` for `2023-07-11 15:27` outputs `2023/07/11 15:27`
- [ ] Format specifiers that should NOT pad (e.g., `%-m`, `%-d`) still produce unpadded output if supported
- [ ] muan-blog DOM match improves significantly (673+ pages fixed)
- [ ] `cargo test` passes

## Test Scenarios

### Unit: Leading zero padding (write FIRST, must fail before fix)

- **Test `test_date_format_month_leading_zero`**: Format date `2023-07-11` with `%m`. Assert output is `07`.
- **Test `test_date_format_day_leading_zero`**: Format date `2023-07-05` with `%d`. Assert output is `05`.
- **Test `test_date_format_hour_leading_zero`**: Format datetime with hour 3 using `%H`. Assert output is `03`.
- **Test `test_date_format_combined`**: Format `2023-07-11T15:27:00` with `%Y/%m/%d %H:%M`. Assert `2023/07/11 15:27`.
- **Test `test_date_format_double_digit_no_change`**: Format date `2023-12-25` with `%m/%d`. Assert `12/25` (already two digits, unchanged).

### Regression: Other date formats preserved

- **Test `test_date_format_year_unchanged`**: `%Y` still produces 4-digit year.
- **Test `test_date_format_textual_month`**: `%B` still produces full month name (e.g., "July").

### Integration: Output verification

- Build muan-blog and inspect story pages to verify dates have leading zeros.
