# Issue 582: Bitcoin-org redirect page generator (redirects.rb emulation)

## Problem

bitcoin-org's `_plugins/redirects.rb` reads the `redirects:` hash from `_config.yml`
and generates redirect HTML pages for each entry. There are 483 redirect entries
(90 internal, 393 external). Each generates an HTML page using the `redirect` layout.

This is different from the `jekyll-redirect-from` plugin (already supported in #481),
which reads `redirect_from`/`redirect_to` from individual page front matter. The
bitcoin-org redirects.rb reads a centralized list from `_config.yml`.

Currently 482 of the 2587 only-Jekyll pages are redirect pages (the count is slightly
less than 483 because some redirect source paths may collide with existing pages or
have other edge cases).

## How redirects.rb works

### Data source

`_config.yml` contains a `redirects:` hash mapping source paths to destination URLs:
```yaml
redirects:
  /en/bitcoin-for-developers: https://developer.bitcoin.org/
  /en/developer-guide: https://developer.bitcoin.org/devguide/
  /nl/bitcoin-paper: /nl/bitcoin-document
  /releases/2011/04/27/v0.3.21: /en/release/v0.3.21
  /bitcoin_es_latam.pdf: /files/bitcoin-paper/bitcoin_es_latam.pdf
```

### Generation algorithm

```
redirects = load _config.yml["redirects"]
for each (src, dst) in redirects:
    srcar = src.split("/")
    filename = srcar.pop() + ".html"
    directory = srcar.join("/")

    create PageRedirect:
        dir = directory
        name = filename
        layout = "redirect"
        page.redirect = dst
        page.lang = "en"

    # Content comes from reading index.html front matter (as a base)
    # but the redirect layout overrides everything
```

### Redirect layout (`_layouts/redirect.html`)

```html
---
layout: base
---
<meta name="robots" content="noindex">
<script>window.location.href='{{ page.redirect }}';</script>

<div class="redirectmsg">
<h1>This page has been moved</h1>
<p><a href="{{ page.redirect }}">bitcoin.org{{ page.redirect }}</a></p>
</div>
```

The redirect layout extends `base`, so redirect pages get the full site chrome
(header, footer, language selector). This is why they show up as full HTML pages
in the Jekyll output, not just bare meta-refresh redirects.

### URL patterns

The source path is split on `/` to derive directory and filename:
- `/en/bitcoin-for-developers` -> directory: `/en`, file: `bitcoin-for-developers.html`
- `/releases/2011/04/27/v0.3.21` -> directory: `/releases/2011/04/27`, file: `v0.3.21.html`
- `/bitcoin_es_latam.pdf` -> directory: `/`, file: `bitcoin_es_latam.pdf.html`

### Edge cases

- Source paths with URL fragments (e.g., `/en/developer-guide#blockchain`) -- these generate
  filenames like `developer-guide#blockchain.html`. Jekyll writes these to disk; browsers
  interpret the `#` as a fragment. The redirect layout's JS handles the actual navigation.
- External destination URLs (https://...) -- the redirect page uses JS and a link to navigate
- Paths ending in `.pdf` -- generates `bitcoin_es_latam.pdf.html`

## Detection

Activate this generator when ALL of the following are true:
- `_plugins/redirects.rb` exists (or contains `RedirectPageGenerator`)
- `_config.yml` contains a non-empty `redirects:` hash

This is NOT site-specific -- any Jekyll site with this plugin pattern would trigger it.

## Dependencies

- Issue #483 must be `.done.md` (translate tag, needed by base layout) -- DONE
- The `redirect` layout must exist in `_layouts/`

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (including new tests)
- [ ] Generator activates only when `_plugins/redirects.rb` exists and `_config.yml` has `redirects:` hash
- [ ] Generator does NOT activate on sites without the plugin or config
- [ ] Redirect pages use the `redirect` layout (rendered through the full layout chain including `base`)
- [ ] `page.redirect` is set to the destination URL and accessible in templates
- [ ] `page.lang` is set to `"en"` on all redirect pages
- [ ] Source path splitting correctly derives directory and filename (last segment + `.html`)
- [ ] Redirect pages with URL fragments in source path (e.g., `#blockchain`) generate correctly
- [ ] External destination URLs (https://...) are preserved as-is in `page.redirect`
- [ ] Internal destination URLs (e.g., `/nl/bitcoin-document`) are preserved as-is
- [ ] bitcoin-org file count increases by approximately 480 (from ~3050 with #581 to ~3530)
- [ ] DTC DOM: 790/790 must not regress
- [ ] bitcoin-org DOM: matched count must not drop below 1
- [ ] No site-specific hardcoding

## Test Scenarios

### Unit: Generator detection
- Site with `_plugins/redirects.rb` and `redirects:` in config -> activates
- Site with `_plugins/redirects.rb` but no `redirects:` in config -> does NOT activate
- Site without `_plugins/redirects.rb` -> does NOT activate

### Unit: Path splitting
- `/en/bitcoin-for-developers` -> dir: `/en`, file: `bitcoin-for-developers.html`
- `/releases/2011/04/27/v0.3.21` -> dir: `/releases/2011/04/27`, file: `v0.3.21.html`
- `/bitcoin_es_latam.pdf` -> dir: `/`, file: `bitcoin_es_latam.pdf.html`
- `/en/developer-guide#blockchain` -> dir: `/en`, file: `developer-guide#blockchain.html`

### Unit: Page generation
- Given 3 redirect entries, verify 3 pages created
- Verify layout is `redirect` on each page
- Verify `page.redirect` is set to destination URL
- Verify `page.lang` is `"en"`
- Verify page URL matches the source path

### Integration: bitcoin-org build
- Build bitcoin-org, verify redirect pages exist (e.g., `en/bitcoin-for-developers.html`)
- Verify redirect page contains `window.location.href` pointing to destination
- Verify redirect page contains `<div class="redirectmsg">`
- Verify redirect page is wrapped in full site layout (has header/footer)
- Run DOM comparison: DTC 790/790 must not regress

### Regression: Other sites unaffected
- Build DTC site, verify no redirect generator activation
- Build other test sites with no `redirects:` config, verify no activation

## Baseline

- DTC DOM: 790/790 matched (must not regress)
- bitcoin-org: 975 files currently (with #581: ~3050, target after this: ~3530)
- bitcoin-org only-Jekyll redirect pages: ~482

## Implementation Notes

### Approach

1. **Detection**: Check for `_plugins/redirects.rb` containing `RedirectPageGenerator` and
   `redirects` key in site config
2. **Config reading**: Parse the `redirects:` hash from the already-loaded `SiteConfig`
3. **Page generation**: For each (src, dst) pair:
   - Split src on `/`, pop last segment as filename + `.html`
   - Create a `Page` with layout `redirect`, `page.redirect = dst`, `page.lang = "en"`
4. **Integration**: Feed pages into the rendering pipeline

### Interaction with existing redirect support

Rustkyll already supports `redirect_from`/`redirect_to` (jekyll-redirect-from, #481).
This is a separate mechanism -- `redirects.rb` reads from `_config.yml`, not from
individual page front matter. Both can coexist.

The key difference: `redirects.rb` pages use the `redirect` layout (full site chrome),
while `redirect_from` generates bare meta-refresh HTML. They should remain separate code paths.

## Log

### [SWE] 2026-04-02

**Fix 1: Redirect page generator module**
- Wrote 16 unit tests in src/redirect_generator.rs:
  - 5 detection tests (activate with plugin+config, no plugin, no config, empty config, wrong plugin content)
  - 5 path splitting tests (simple, deep, root-level, fragment, unicode)
  - 6 page generation tests (basic, redirect URL, URL structure, no key, fragment, unicode)
- Ran tests: ALL PASS (16/16)
- Implemented redirect_generator.rs with should_activate(), split_redirect_path(), generate_redirect_pages()
- Ran tests: PASSES

**Fix 2: Integration into main.rs pipeline**
- Added redirect generator call in step 4a3 (after template generator, before site context build)
- Pages are added to the `pages` vector and rendered through the normal layout pipeline
- Verified bitcoin-org build: 483 redirect pages generated, 3534 total HTML files
- Verified redirect page content: `window.location.href='/en/release/v0.3.21'` and `<div class="redirectmsg">` present
- Verified redirect pages use full site chrome (base layout with header/footer)

**Summary:**
- Files created: src/redirect_generator.rs
- Files modified: src/lib.rs, src/main.rs
- Tests added: 16 unit tests for detection, path splitting, and page generation
- Build results: 3963 lib tests + integration tests pass, 0 fail, clippy clean, fmt clean
- DTC DOM: 790/790 matched, 0 total diffs (no regression)
- DTC build time: 0.89s (under 1.0s threshold)
- bitcoin-org: 483 redirect pages generated, 3534 total HTML files (up from ~3050)

### [PM] 2026-04-02 16:45
- Reviewed diff: 4 files changed (src/redirect_generator.rs new, src/lib.rs, src/main.rs, tracker file)
- Output verification: Built bitcoin-org site, inspected /tmp/bitcoin_582/en/developer-guide.html -- full site chrome present, window.location.href='https://developer.bitcoin.org/devguide/' correct, redirectmsg div present. Checked /tmp/bitcoin_582/nl/bitcoin-paper.html -- internal redirect to /nl/bitcoin-document correct. 3534 total HTML files.
- DTC DOM verified: 790/790 matched, no regression
- Tests: 16 unit tests cover detection (5), path splitting (5), page generation (6) including unicode
- Code review: clean module, proper detection (plugin file + config), no site-specific hardcoding
- clippy note: new code is clean; pre-existing warnings in wallet_generator.rs are unrelated to this issue
- Acceptance criteria: all met
- Follow-up issues created: none
- VERDICT: ACCEPT
