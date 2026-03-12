# Issue 07: Template Filters

## Description

Implement the Liquid filters used throughout the Jekyll site templates. These transform values in `{{ expression | filter }}` chains.

## Dependencies

- Issue 06 (template engine core)

## Scope

- Array filters: `where`, `where_exp`, `sort`, `reverse`, `map`, `uniq`, `first`, `last`, `size`, `join`, `push`, `slice`, `compact`
- String filters: `append`, `prepend`, `default`, `strip`, `strip_html`, `strip_newlines`, `truncate`, `slugify`, `markdownify`, `newline_to_br`, `split`
- Date filters: `date_to_string`, `date_to_xmlschema`
- JSON filter: `jsonify`
- URL filter: `relative_url`
- Math filters: `plus`, `minus`, `times`, `divided_by`, `modulo`
- Unit tests for each filter with real data from the site
