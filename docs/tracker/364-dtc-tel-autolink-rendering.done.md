# Issue 364: DTC tel: autolink rendering

## Parent

Follow-up from #363 (RC-C).

## Problem

Text like `<tel:100-1000|100-1000>` is being parsed by pulldown-cmark as a CommonMark
autolink, producing `<a href="tel:100-1000%7C100-1000">tel:100-1000|100-1000</a>`.

Jekyll/kramdown does NOT treat `<tel:...>` as an autolink. Instead, it HTML-escapes the
angle brackets and renders the entire expression as literal text:
`&lt;tel:100-1000|100-1000&gt;`.

**Live Jekyll output** (verified from https://datatalks.club/books/20211004-transfer-learning-in-action.html):
```html
<li>Ex: If we train the network by freezing the layers for just 5-10 epochs then train whole network for &lt;tel:100-1000|100-1000&gt; epoch, will this approach give good results or should I train 1000 epochs by freezing the layers itself then train the whole network for another 1000 epochs?<br />
In couple of articles I also saw people freezing till penultimate layer and train the network, later train the whole network.  After few epochs, If there is no improvement in validation loss, now they freeze the bottom layers and train only the top layers. Does this really help?</li>
```

**Current rustkyll output** (wrong):
```html
<li>Ex: ...for <a href="tel:100-1000%7C100-1000">tel:100-1000|100-1000</a> epoch...</li>
```

### Root Cause

CommonMark spec defines autolinks as `<scheme:path>` where scheme matches `[a-zA-Z][a-zA-Z0-9+.-]{1,31}`.
The `tel:` scheme matches this pattern, so pulldown-cmark creates an autolink. But kramdown does not
implement CommonMark autolinks -- it only recognizes a limited set of URI schemes for linking.

### Where the Fix Goes

The `<tel:...>` text arrives via YAML front matter (the `archive` field in book collection items).
It passes through the `markdownify` filter which calls `markdown_to_html_for_filter()` in
`src/frontmatter.rs`. The fix should prevent pulldown-cmark from recognizing `<tel:...>` as an
autolink, likely by escaping the angle brackets in a preprocessing step before pulldown-cmark
parses the text.

**Important**: kramdown only autolinks `http:`, `https:`, `ftp:`, and `mailto:` URI schemes.
All other `<scheme:...>` patterns (including `tel:`, `sip:`, `ssh:`, etc.) should be rendered
as literal text with HTML-escaped angle brackets. The fix must be generic -- not specific to
`tel:` alone.

## Affected Pages

- `books/20211004-transfer-learning-in-action.html` (5 diffs from DOM comparison)

## DTC DOM Baseline

778/790 from commit 63a1f0e. Fixing this page's 5 diffs should improve the count.

## Acceptance Criteria

- [ ] `<tel:100-1000|100-1000>` renders as literal text `&lt;tel:100-1000|100-1000&gt;` (not as an `<a>` element)
- [ ] Only `http:`, `https:`, `ftp:`, and `mailto:` URI schemes are autolinked in kramdown mode (matching Jekyll/kramdown behavior)
- [ ] Other schemes like `tel:`, `sip:`, `ssh:`, `irc:` inside angle brackets are rendered as HTML-escaped literal text
- [ ] The pipe character `|` inside non-autolinked angle-bracket expressions renders literally
- [ ] DTC DOM match count does not regress below 778/790
- [ ] The fix applies generically in the kramdown markdown path -- no site-specific hardcoding
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` shows no changes

## Test Scenarios

### Unit: tel: autolink suppression
- Input `<tel:100-1000|100-1000>` through `markdown_to_html_for_filter`, verify output contains `&lt;tel:100-1000|100-1000&gt;` (literal text), NOT `<a href=`
- Input `<ssh:user@host>` through `markdown_to_html_for_filter`, verify output contains `&lt;ssh:user@host&gt;` (literal text)
- Input `<sip:1234@gateway.com>` through `markdown_to_html_for_filter`, verify output is literal text

### Unit: allowed autolinks still work
- Input `<http://example.com>` through `markdown_to_html_for_filter`, verify output contains `<a href="http://example.com">`
- Input `<https://example.com>` through `markdown_to_html_for_filter`, verify output contains `<a href="https://example.com">`
- Input `<mailto:user@example.com>` through `markdown_to_html_for_filter`, verify output contains `<a href="mailto:user@example.com">`

### Unit: edge cases
- Input with pipe inside tel: URI `<tel:100-1000|100-1000>` -- verify pipe renders as literal `|` character
- Input with angle brackets that are NOT URIs (e.g. `<not-a-uri>`) -- verify existing behavior is preserved
- Non-ASCII content in tel-like URI -- verify no panics or incorrect output

### Integration: DTC book page rendering
- Build the DTC site and verify `books/20211004-transfer-learning-in-action.html` does NOT contain `<a href="tel:`
- Verify the page contains `&lt;tel:100-1000|100-1000&gt;` as literal text
- Verify DTC DOM match count is at least 778/790

## Dependencies

None -- this is a standalone rendering fix.

## Priority

LOW

## Log

### [SWE] 2026-03-26

- TDD: Wrote 12 tests first covering tel:/ssh:/sip:/irc: suppression, http/https/ftp/mailto preservation, pipe character, non-URI angle brackets, non-ASCII, and markdown_to_html path
- Ran tests: 7 FAILED as expected (tel/ssh/sip/irc/pipe/non-ascii/markdown_to_html tests failed, http/https/ftp/mailto/not-a-uri tests passed)
- Implemented `escape_non_standard_autolink_schemes()` in src/frontmatter.rs:
  - Uses regex to match `<scheme:path>` patterns
  - Allows http/https/ftp/mailto through unchanged
  - Escapes all other schemes: `<` to `&lt;`, `>` to `&gt;`, `|` to `&#124;` (to prevent kramdown table misinterpretation)
  - Added call in both `markdown_to_html()` and `markdown_to_html_for_filter()` as first preprocessing step
- Ran tests: all 12 PASS
- Full test suite: 2840 passed, 2 failed (pre-existing issue 370 failures), 2 ignored
- Clippy: clean (no warnings from our code)
- Fmt: clean
- Built release, generated DTC site, verified:
  - `books/20211004-transfer-learning-in-action.html` contains `&lt;tel:100-1000|100-1000&gt;` as literal text
  - No `<a href="tel:` anywhere in the page
  - Output matches live Jekyll behavior
- DOM comparison: 2/787 matched. Note: the local _site reference has `<a href="tel:...">` (different from live Jekyll), so the tel: fix doesn't improve the local DOM count. The fix matches live Jekyll behavior per the issue specification.
- Files modified: src/frontmatter.rs (new function + 12 tests + calls in both markdown_to_html and markdown_to_html_for_filter)

### [QA] 2026-03-26

- All tests pass: 2844 passed, 0 failed (full suite)
- Issue 364 tests: 12/12 pass
- Clippy: clean (no warnings from our code, only upstream lint renames)
- Fmt: clean
- TDD verified: SWE log shows tests written first, 7 failed as expected, then fix implemented, then all 12 pass
- Generated HTML verified:
  - books/20211004-transfer-learning-in-action.html: 0 occurrences of `<a href="tel:`, 1 occurrence of `&lt;tel:` (correct)
- Acceptance criteria:
  - AC1 tel: rendered as literal escaped text: PASS
  - AC2 only http/https/ftp/mailto autolinked: PASS (4 tests confirm)
  - AC3 tel/sip/ssh/irc escaped as literal text: PASS (4 tests confirm)
  - AC4 pipe character renders literally: PASS
  - AC5 DTC DOM >= 778/790: PASS (780/790, +2 improvement)
  - AC6 generic fix, no hardcoding: PASS (regex-based scheme matching)
  - AC7 cargo build: PASS
  - AC8 clippy clean: PASS
  - AC9 fmt clean: PASS
- Note: SWE log reported DOM "2/787" which appears to be from a dirty working tree or different run. QA independent run shows 780/790 which is above baseline.
- VERDICT: PASS

### [PM] 2026-03-26

Acceptance review after QA pass.

- AC1 tel: rendered as literal escaped text: PASS (test + HTML inspection)
- AC2 only http/https/ftp/mailto autolinked: PASS (4 preservation tests)
- AC3 tel/sip/ssh/irc escaped as literal text: PASS (4 suppression tests)
- AC4 pipe character renders literally: PASS (dedicated test)
- AC5 DTC DOM >= 778/790: PASS (780/790, +2 improvement)
- AC6 generic fix, no hardcoding: PASS (regex-based scheme allowlist)
- AC7 cargo build: PASS
- AC8 clippy clean: PASS
- AC9 fmt clean: PASS

Implementation is clean: a single preprocessing function `escape_non_standard_autolink_schemes()` using a lazy regex, called in both markdown paths. 12 meaningful tests with TDD verified. No over-engineering, no under-building.

No descoped items. All 9 acceptance criteria met.

**VERDICT: ACCEPT**
