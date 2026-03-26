# Issue 367: DTC URL asterisk rendering in markdown

## Parent

Follow-up from #363 (RC-F).

## Problem

O'Reilly URLs containing `*` characters (e.g., `_gl=1*95hemv*_ga*MTA2...`) are being parsed as `<em>` emphasis markers instead of literal characters within the URL text.

The specific pattern is a markdown link where the link text itself is a URL containing asterisks:

```
[https://www.oreilly.com/.../?_gl=1*95hemv*_ga*MTA2...](https://www.oreilly.com/.../?_gl=1*95hemv*_ga*MTA2...)
```

Jekyll/kramdown treats `*` inside `[...]` link text as literal when the text looks like a URL. Our span parser incorrectly interprets `*95hemv*` as `<em>95hemv</em>`, which cascades into 13+ DOM differences on the page.

## Affected Pages

- `books/20221121-reliable-machine-learning.html` (13 of its 15 diffs are caused by this bug)

## Source File

- `websites/DataTalksClub/datatalksclub.github.io/_books/20221121-reliable-machine-learning.md`
- The problematic text is in the `archive:` YAML section, in a reply containing a bare O'Reilly URL with `_gl=1*95hemv*_ga*MTA2ODM2NTQzNi4xNjU1NjQ3NTg4*_ga_092EL089CH*...` query parameters.

## Root Cause Area

`src/kramdown_parser/span_parser.rs` -- the emphasis parsing logic (`try_parse_emphasis` and related functions). When inside `[...]` link text, asterisks that are part of URL query parameters should not trigger emphasis parsing.

Kramdown's rule: asterisks flanked by non-whitespace on both sides inside link text containing URL-like content should be treated as literal `*` characters, not emphasis markers.

## Dependencies

None (no other `.in-progress.md` or `.groomed.md` issues block this).

## DTC DOM Baseline

780/790 matched (from commit `bd99515`, issue #370).

## Acceptance Criteria

- [ ] Asterisks inside URL link text `[url*with*stars](...)` are not parsed as emphasis markers
- [ ] The reliable-machine-learning.html page renders the O'Reilly URL as plain text without `<em>` tags, matching Jekyll output
- [ ] Build the DTC site and verify `books/20221121-reliable-machine-learning.html` -- the paragraph containing the O'Reilly link must not contain spurious `<em>` elements
- [ ] DTC DOM match count does not drop below 780/790
- [ ] Fix is generic (applies to any URL containing asterisks in link text, not hardcoded to O'Reilly)
- [ ] No site-specific hardcoding
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (all existing tests plus new ones)
- [ ] `cargo clippy -- -D warnings` is clean
- [ ] `cargo fmt` produces no changes

## Test Scenarios

### Unit: Asterisks in URL link text

- Parse `[https://example.com/?a=1*foo*bar](https://example.com/?a=1*foo*bar)` -- verify output contains no `<em>` tags, asterisks render as literal `*`
- Parse `[https://site.com/?_gl=1*abc*_ga*123](url)` -- verify `*abc*` is NOT wrapped in `<em>`
- Parse `text *emphasis* more` -- verify `<em>emphasis</em>` still works (regression check)
- Parse `[regular *emphasis* in link](url)` -- verify emphasis still works inside non-URL link text
- Parse text with Unicode characters mixed with asterisk URLs -- verify no encoding issues

### Integration: DTC reliable-machine-learning page

- Build the DTC site (`websites/DataTalksClub/datatalksclub.github.io`)
- Inspect `books/20221121-reliable-machine-learning.html` output
- Verify the paragraph containing the O'Reilly fairness book URL does not contain `<em>95hemv</em>` or similar
- Verify the URL text renders as a single unbroken string with literal asterisks
- Run DOM comparison and confirm no regression below 780/790

## Output Verification

After building the DTC site, the generated `books/20221121-reliable-machine-learning.html` must contain the O'Reilly URL rendered as literal text. Specifically:

- The text `_gl=1*95hemv*_ga*MTA2ODM2NTQzNi4xNjU1NjQ3NTg4` must appear as-is, not split by `<em>` tags
- There must be zero `<em>` elements wrapping URL query parameter fragments like `95hemv` or `MTA2ODM2NTQzNi4xNjU1NjQ3NTg4`

## Priority

LOW
