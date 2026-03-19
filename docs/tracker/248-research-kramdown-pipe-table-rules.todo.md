# Issue 248: Research kramdown pipe table extension rules

## Problem

Descoped from issue 227 (pattern 2). The mlwiki.org site has lines with pipe characters (`| A | B | C |`) that pulldown-cmark parses as tables but kramdown sometimes does NOT.

Issue 227's implementation assumed kramdown always requires a header separator row (`|---|---|`) for pipe tables. This was WRONG -- kramdown's pipe table extension sometimes renders pipe-only lines as tables even without separator rows. The aggressive escaping removed 105 original diffs but introduced 182 NEW diffs (net regression of 77).

## Required Research

1. Read kramdown's pipe table parser source code (`lib/kramdown/parser/kramdown/table.rb`) to understand the actual rules
2. Test specific patterns against kramdown directly to determine:
   - Does `| A | B |\n` alone (no separator) become a table? Under what conditions?
   - Does `| A | B |\n|---|---|\n| 1 | 2 |` -- standard table?
   - Does `| A | B |\nplain text` -- does the lack of separator prevent table parsing?
   - Does context matter (inside list items, blockquotes, etc.)?
   - What about MediaWiki-style `||` double-pipe separators?
3. Document the exact rules with test cases
4. Only then implement a fix that correctly matches kramdown's behavior

## Estimated Impact

~600-900 diffs on mlwiki.org (including cascade effects from wrong table parsing).

## Dependencies

- None. This can be worked independently.
