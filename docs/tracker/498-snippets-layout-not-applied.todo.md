# Issue 498: snippets site — layout not applied to 17 pages

## Problem

snippets (8/25) has 17 pages missing `<head>/<body>` wrapper despite
having local _layouts/ with default.html, snippet.html, category.html.
No render errors logged. Liquid rendering fails silently.

## Scope

Investigate why layouts are not applied. Check if Liquid template
parsing fails for these specific pages (like #441 found for other sites).

## Baseline

DTC 790/790. snippets 8/25. Must not regress.
