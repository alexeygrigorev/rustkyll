# Issue 460: muan-blog escaped dash \- becomes <hr>

## Problem

2 muan-blog pages have `\-` at line start being converted to `<hr>`
instead of a literal `-`. Jekyll treats `\-` as an escaped dash.
2 diffs, 2 files.

## Scope

Fix escaped dash handling in markdown preprocessing to produce
literal `-` instead of triggering horizontal rule.

## Baseline

DTC 790/790. DTC docs 48/57. muan-blog 2196/2218.
