# Issue 496: Kramdown inline attribute lists {: .class :}

## Problem

Kramdown inline attribute lists (IALs) like `{: .mx-auto.d-block :}` are
rendered as literal text instead of being applied as HTML class attributes
on the preceding element.

Jekyll/kramdown: `<img class="d-block mx-auto" ...>`
Rustkyll: `<img ...>\n{: .mx-auto.d-block :}`

## Affected Sites

- beautiful-jekyll (1 page, sample-markdown)
- Potentially any site using kramdown IALs

## Scope

Parse `{: .class#id attr=val :}` patterns after HTML elements and apply
them as attributes. This is a kramdown-specific feature.

## Baseline

DTC 790/790. beautiful-jekyll 4/5. Must not regress.
