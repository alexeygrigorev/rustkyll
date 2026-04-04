# Issue 583: Bitcoin-org alert shorturl page generation

## Problem

bitcoin-org's `_plugins/alerts.rb` generates the main alert pages at `/en/alert/{filename}`
(already working via `output: true` on the alerts collection). However, the generator
also creates **shorturl alias pages** for alerts that have a `shorturl` front matter
variable. Each shorturl generates 2 extra pages:

- `/{shorturl}.html` (e.g., `/android.html`)
- `/{shorturl}/index.html` (e.g., `/android/index.html`)

These shorturl pages use the `alert` layout but include a canonical link and JS redirect
pointing to the main alert URL (e.g., `/en/alert/2013-08-11-android`).

All 14 alerts (out of 15 total; one alert `2014-02-11-malleability.html` has no shorturl
but does have it -- actually all 14 have shorturls) have `shorturl` defined, producing 28
shorturl pages. These 28 pages are currently missing from rustkyll output.

## How alerts.rb shorturl works

When processing an alert file, if `shorturl` is defined in front matter:

```ruby
if self.data.has_key?('shorturl')
  # Create /{shorturl}.{extension} at root
  site.pages << AlertPage.new(site, base, lang, srcdir, src, '', self.data['shorturl']+'.'+extension, date)
  # Create /{shorturl}/index.{extension} at root
  site.pages << AlertPage.new(site, base, lang, srcdir, src, '', self.data['shorturl']+'/index.'+extension, date)
end
```

When `dstdir == ''` (root-level shorturl page), the AlertPage sets:
- `page.canonical = '/en/alert/' + src.split('.')[0]`
- Does NOT set `page.category = 'alert'`
- Does NOT process banner/active/shorturl (avoids recursion)

The shorturl pages render with the `alert` layout, which checks for `page.canonical`:
```html
{% if page.canonical != nil %}
<script>window.location.href='{{ page.canonical }}';</script>
<link rel="canonical" href="https://bitcoin.org{{ page.canonical }}"/>
{% endif %}
```

### Shorturl mappings (from alert front matter)

| Alert file | shorturl | Pages generated |
|-----------|----------|----------------|
| 2012-02-18-protocol-change.html | feb20 | feb20.html, feb20/index.html |
| 2012-03-16-critical-vulnerability.html | critfix | critfix.html, critfix/index.html |
| 2012-05-14-dos.html | dos | dos.html, dos/index.html |
| 2013-03-11-chain-fork.html | chainfork | chainfork.html, chainfork/index.html |
| 2013-03-15-upgrade-deadline.html | may15 | may15.html, may15/index.html |
| 2013-08-11-android.html | android | android.html, android/index.html |
| 2014-04-11-heartbleed.html | heartbleed | heartbleed.html, heartbleed/index.html |
| 2015-07-04-spv-mining.md | spv-mining | spv-mining.html, spv-mining/index.html |
| 2015-10-12-upnp-vulnerability.md | upnp-vulnerability | upnp-vulnerability.html, upnp-vulnerability/index.html |
| 2016-08-17-binary-safety.md | binary-safety | binary-safety.html, binary-safety/index.html |
| 2016-11-01-alert-retirement.md | alert-retirement | alert-retirement.html, alert-retirement/index.html |
| 2017-07-12-potential-split.md | potential-split | potential-split.html, potential-split/index.html |
| 2017-10-09-segwit2x-safety.md | segwit2x-safety | segwit2x-safety.html, segwit2x-safety/index.html |
| 2018-09-21-required-upgrade.md | required-upgrade | required-upgrade.html, required-upgrade/index.html |

Total: 14 alerts x 2 pages = 28 pages.

## Detection

Activate this feature when:
- `_plugins/alerts.rb` exists (or contains `AlertPageGenerator`)
- The `alerts` collection exists with `output: true`
- Alert documents have `shorturl` in front matter

This extends the existing collection output behavior -- alert pages are already
rendered, this adds the shorturl alias pages.

## Dependencies

- Issue #483 must be `.done.md` (translate tag, needed by alert layout) -- DONE
- The `alert` layout must exist in `_layouts/`
- Alerts collection must already be rendered (currently working)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (including new tests)
- [ ] For each alert with `shorturl: X` in front matter, two pages are generated: `/{X}.html` and `/{X}/index.html`
- [ ] Shorturl pages use the `alert` layout
- [ ] Shorturl pages have `page.canonical` set to `/en/alert/{filename_without_extension}`
- [ ] Shorturl pages do NOT have `page.category` set to `alert`
- [ ] Shorturl pages have `page.lang` set to `"en"`
- [ ] Shorturl pages have `page.date` set to the alert's date (parsed from filename)
- [ ] All 28 shorturl pages are generated for bitcoin-org (14 alerts x 2)
- [ ] The generated HTML contains `window.location.href` pointing to the canonical alert URL
- [ ] The generated HTML contains `<link rel="canonical">` pointing to the canonical alert URL
- [ ] DTC DOM: 790/790 must not regress
- [ ] bitcoin-org DOM: matched count must not drop below 1
- [ ] No site-specific hardcoding

## Test Scenarios

### Unit: Shorturl detection
- Alert with `shorturl: "android"` -> generates 2 pages
- Alert without `shorturl` in front matter -> no extra pages
- Alert with empty `shorturl: ""` -> no extra pages

### Unit: Page properties
- Shorturl page at `/{shorturl}.html`: verify layout is `alert`, page.canonical is `/en/alert/{date-slug}`
- Shorturl page at `/{shorturl}/index.html`: verify same properties
- Verify `page.category` is NOT set on shorturl pages
- Verify `page.date` is extracted from alert filename
- Verify file extension matches source (.html or .md alerts both produce .html output)

### Unit: Canonical URL construction
- Alert `2013-08-11-android.html` with `shorturl: "android"` -> canonical: `/en/alert/2013-08-11-android`
- Alert `2015-07-04-spv-mining.md` with `shorturl: "spv-mining"` -> canonical: `/en/alert/2015-07-04-spv-mining`

### Integration: bitcoin-org build
- Build bitcoin-org, verify `android.html` and `android/index.html` exist at site root
- Verify `feb20.html` exists (shorturl from .html source)
- Verify `spv-mining.html` exists (shorturl from .md source)
- Verify shorturl page HTML contains JS redirect to canonical URL
- Count: all 28 shorturl pages present
- Run DOM comparison: DTC 790/790 must not regress

### Regression
- Build DTC site (no alerts collection) -> no shorturl pages generated
- Existing alert pages at `/en/alert/*` are not affected

## Baseline

- DTC DOM: 790/790 matched (must not regress)
- bitcoin-org: 975 files currently, 28 shorturl pages missing
- After #581 + #582 + this: target ~3560 files (closing most of the 3562 gap)

## Implementation Notes

### Approach

After rendering alert collection items (which already happens because `output: true`),
scan each alert's front matter for `shorturl`. For each shorturl found:

1. Create two virtual pages:
   - URL: `/{shorturl}.html` (or `/{shorturl}.md` -> `.html`)
   - URL: `/{shorturl}/index.html`
2. Set page properties: `layout: alert`, `lang: "en"`, `canonical: /en/alert/{slug}`, `date: parsed`
3. Do NOT set `category: alert` on shorturl pages
4. Feed through normal rendering pipeline

### Integration point

This can be implemented as a post-processing step after collection loading but before
rendering, or as part of the collection rendering loop. The key is that shorturl pages
need the same content as the original alert but different metadata (canonical, no category).

## Log

### [SWE] 2026-04-02

**Fix 1: Alert shorturl page generation**
- Wrote 13 unit tests in src/alert_shorturl_generator.rs covering:
  - Detection (should_activate with/without plugin, with/without output:true, with/without alerts collection)
  - Generation (alert with shorturl generates 2 pages, without shorturl generates 0, empty shorturl generates 0)
  - Page properties (layout=alert, lang=en, canonical URL, no category, date)
  - Canonical URL from .html and .md sources
  - Multiple alerts correct count
  - Unicode title preservation
  - All 14 bitcoin-org alerts produce 28 pages
- Ran tests: all 13 PASS
- Implemented alert_shorturl_generator.rs module with should_activate() and generate_shorturl_pages()
- Wired into main.rs after redirect_generator (step 4a5)
- Registered module in lib.rs

**Verification:**
- bitcoin-org build: 28 shorturl pages generated
- Verified android.html exists with window.location.href='/en/alert/2013-08-11-android' and canonical link
- DTC DOM: 790/790 matched, 0 total diffs (no regression)
- DTC build time: 0.876s (under 1.0s)
- Full test suite: all tests pass (3998+ unit + integration)
- clippy: clean (no warnings)
- fmt: clean

**Summary:**
- Files created: src/alert_shorturl_generator.rs
- Files modified: src/lib.rs (added module), src/main.rs (wired generator)
- Tests added: 13 unit tests
- Build results: all tests pass, clippy clean, fmt clean

### [PM] 2026-04-05 10:00
- Reviewed diff: 1 new file (src/alert_shorturl_generator.rs, 587 lines), tracker file update
- lib.rs and main.rs wiring already committed in #581
- Output verification:
  - Built DTC site: 790/790 DOM match (no regression)
  - Built bitcoin-org site: 5054 files generated
  - All 28 shorturl pages verified present (14 root .html + 14 index.html)
  - Inspected android.html content: contains window.location.href='/en/alert/2013-08-11-android' and canonical link
  - Inspected android/index.html: same redirect + canonical present
- Tests: 13 unit tests covering detection, generation, page properties, canonical URLs, unicode, and full 14-alert count
- Code review: clean implementation, detection via _plugins/alerts.rb (no hardcoding), proper use of Result/Option
- Acceptance criteria: all met
- VERDICT: ACCEPT
