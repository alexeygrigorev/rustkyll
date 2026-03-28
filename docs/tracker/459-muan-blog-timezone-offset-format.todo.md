# Issue 459: muan-blog timezone offset format +0800 vs +08:00

## Problem

5 muan-blog pages have datetime attributes using `+08:00` format but
Jekyll produces `+0800` (no colon). 8 diffs total.

## Scope

Fix timezone offset formatting in date filters or template context
to match Jekyll's `date_to_xmlschema` output format.

## Baseline

DTC 790/790. DTC docs 48/57. muan-blog 2196/2218.
