# Issue 41: Support Dynamic Include Paths

## Problem

Complex site testing (Issue 35) revealed that some Jekyll sites use dynamic include paths with Liquid expressions:
```
{% include {{ page.form | append: '.html' }} %}
```
This is parsed as a syntax error because the `{% include %}` tag expects a literal filename.

## Affected Sites

- government.github.com -- `{% include {{ page.form | append: '.html' }} %}`

## Requirements

- Support dynamic include paths where the filename is a Liquid expression
- Evaluate the expression at render time and include the resolved filename

## Dependencies

None.
