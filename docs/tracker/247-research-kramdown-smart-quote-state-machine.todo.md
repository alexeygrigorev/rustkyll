# Issue 247: Research kramdown smart quote state machine for '' sequences

## Problem

Descoped from issue 227 (pattern 1). The mlwiki.org site uses MediaWiki-style `''italic''` and `'''bold'''` markup (consecutive single quotes). kramdown applies its smart punctuation individually to each `'` character using a context-sensitive state machine, NOT by grouping `''` as a unit.

Issue 227's implementation assumed "first quote straight, rest curly" but this produced wrong byte sequences (e.g., `'\u2019` instead of `\u2018\u2018` for opening `''`). QA found 690 text_differs remain -- essentially no improvement.

## Required Research

1. Read kramdown's actual source code for smart quote handling (Ruby gem, `lib/kramdown/converter/html.rb` or `parser/kramdown/`) to understand the state machine
2. Test specific inputs against kramdown directly (install kramdown gem, run test cases) to get ground truth:
   - `''italic''` -- what exact Unicode codepoints does kramdown produce?
   - `'''bold'''` -- same question
   - `It''s` vs `''It''` -- how does context (preceded by word char vs not) affect the output?
   - Mixed: `The ''cat's'' whiskers` -- how does the apostrophe inside interact?
3. Document the exact transformation rules with codepoint-level precision
4. Only then implement the fix

## Estimated Impact

~690 text_differs on mlwiki.org (the single largest diff category).

## Dependencies

- None. This can be worked independently.
