# Issue 30: Missing Liquid Filters

## Problem

Several common Jekyll/Liquid filters are not implemented: `number_of_words`, `group_by`, `xml_escape`, `sort_natural`, `concat`, `compact`, `truncatewords`.

## Requirements

- Implement `number_of_words` filter (word count)
- Implement `group_by` filter (groups array by property, returns `[{name, items}]`)
- Implement `xml_escape` filter (XML entity encoding)
- Implement any other missing filters encountered during cross-site testing
- All existing tests must continue to pass

## References

- Issue #22 compatibility research, gap #13, #14, #15
