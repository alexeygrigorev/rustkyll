# Issue 36: Revert Book Layout Modification and Generate JSON-LD in Rustkyll

## Problem

Issue #18 added a JSON-LD `Book` + `BreadcrumbList` block directly into `datatalksclub.github.io/_layouts/book.html`. This modifies the reference Jekyll site, which should remain untouched. JSON-LD generation should be handled purely by rustkyll's rendering code.

## Requirements

- Revert `datatalksclub.github.io/_layouts/book.html` to its original state (use `git checkout` on that file)
- Move JSON-LD Book + BreadcrumbList generation into rustkyll's template engine or layout processing
- The generated HTML output must still contain the same JSON-LD structured data as before
- No files in `datatalksclub.github.io/` should be modified by rustkyll development
- All existing JSON-LD tests must continue to pass
- Add a convention/check: rustkyll must never require modifications to the source Jekyll site
