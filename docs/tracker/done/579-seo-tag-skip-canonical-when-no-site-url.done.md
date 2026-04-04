# Issue 579: SEO tag emits canonical/og:url when site.url is not configured

## Problem

When a site does NOT have `url:` in `_config.yml`, Jekyll's jekyll-seo-tag does NOT emit `<link rel="canonical">` or `<meta property="og:url">`. Rustkyll always emits them because `site.url` defaults to `""` (empty string) which is not nil, causing the SEO tag to treat it as a valid (empty) URL and emit `canonical="/"`.

**Jekyll output (no url in config):**
```html
<!-- Begin Jekyll SEO tag v2.8.0 -->
<title>Hacker theme</title>
<meta name="generator" content="Jekyll v4.4.1" />
<meta property="og:title" content="Hacker theme" />
...
<meta name="twitter:card" content="summary" />
<!-- No canonical, no og:url -->
```

**Rustkyll output:**
```html
<!-- Begin Jekyll SEO tag v2.8.0 -->
<title>Hacker theme</title>
<meta name="generator" content="Jekyll v4.4.1" />
<meta property="og:title" content="Hacker theme" />
...
<link rel="canonical" href="/" />           <!-- EXTRA -->
<meta property="og:url" content="/" />      <!-- EXTRA -->
<meta name="twitter:card" content="summary" />
```

The extra tags shift all subsequent elements in `<head>`, causing cascading DOM comparison failures.

## Impact

23 sites don't have `url:` configured. The extra canonical/og:url tags cause DOM comparison diffs on all pages of these sites. Affected sites include:

- **hacker-theme (0/2)**: Only 2 total diffs from this issue. Fix pushes to 2/2 (100%).
- **architect-theme (0/2)**: 10 diffs, primarily from this. Fix likely pushes to 2/2 (100%).
- **merlot-theme (0/2)**: 10 diffs. Fix likely pushes to 2/2 (100%).
- **slate-theme (0/2)**: 10 diffs. Fix likely pushes to 2/2 (100%).
- **time-machine-theme (0/2)**: 10 diffs. Fix likely pushes to 2/2 (100%).
- **cayman-theme, dinky-theme, leap-day-theme, midnight-theme, primer-theme**: All 0/2 with small diff counts -- many likely become 100%.
- **beautiful-jekyll, minima, mediumish, jasper2, documentation-theme-jekyll**: Larger sites that would see improvement.

## Root Cause

In `src/generator.rs` line 333:
```rust
site.insert("url".into(), LiquidValue::scalar(config.url.clone()));
```

When `config.url` is `""` (the default when url is not in _config.yml), this inserts `LiquidValue::scalar("")` which is NOT nil. The SEO tag's `get_nested_str_allow_empty_non_nil` function returns `Some("")` for this, and `canonical_url` becomes `Some("/")`.

The config already tracks `url_explicitly_set` (set to `false` when url is not in _config.yml). This flag is not used when setting `site.url` in the Liquid context.

## Scope

- When `config.url_explicitly_set` is `false`, set `site.url` to nil (not `""`) in the Liquid context
- When `config.url_explicitly_set` is `true` and `config.url` is `""`, set `site.url` to `""` (empty string, as before -- this is the `url: ""` case)
- The SEO tag code at line 611-623 already correctly checks for nil -- no changes needed there
- Verify that `site.url` being nil doesn't break other template usage (e.g., `{{ site.url }}` should render empty, not error)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests plus new ones
- [ ] When `url:` is NOT in `_config.yml`, `site.url` is nil in templates
- [ ] When `url: ""` IS in `_config.yml`, `site.url` is `""` (empty string)
- [ ] When `url: "https://example.com"` IS in config, `site.url` is the URL string
- [ ] Hacker-theme DOM match improves from 0/2 to 2/2 (100%)
- [ ] At least 5 GitHub Pages themes push to 100% DOM match
- [ ] DTC DOM match count must not drop below 790/790
- [ ] `{{ site.url }}` in templates renders empty string (not error) when URL not configured

## Test Scenarios

### Unit: site.url nil vs empty
- Config without `url:` key -> `site.url` is nil in Liquid runtime
- Config with `url: ""` -> `site.url` is `""` in Liquid runtime
- Config with `url: "https://example.com"` -> `site.url` is the URL
- Template `{{ site.url }}` with nil site.url renders empty string

### Unit: SEO tag canonical suppression
- SEO tag with nil `site.url` does NOT output `<link rel="canonical">`
- SEO tag with nil `site.url` does NOT output `<meta property="og:url">`
- SEO tag with `url: ""` DOES output canonical (with relative path)
- SEO tag with full URL DOES output canonical (with absolute URL)

### Integration: Theme site builds
- Build hacker-theme, verify DOM 2/2
- Build architect-theme, verify DOM improvement
- Build DTC, verify 790/790

## Dependencies

None.

## DTC DOM Baseline

790/790 (must not drop)

## Log

### [SWE] 2026-04-02

**Root cause analysis:**

The issue description pointed to `generator.rs` line 333, but investigation revealed that issue 529 already fixed the generator to insert `LiquidValue::Nil` when `url_explicitly_set` is false. The actual root cause was in `LenientValue` (src/template/engine.rs), a wrapper type used for the cached site context. `LenientValue` implements `ValueView` but did NOT override `is_nil()`, which defaults to `false` in the trait. So even though `LenientValue` wrapping `Value::Nil` had `type_name() == "nil"`, calling `is_nil()` returned `false`, preventing the SEO tag's `get_nested_str_allow_empty_non_nil()` from detecting nil values.

**Fix 1: LenientValue::is_nil() delegation**
- Wrote test: test_lenient_value_nil_is_nil (src/template/engine.rs)
- Ran test: FAILS -- "LenientValue wrapping Value::Nil must return is_nil() == true"
- Implemented fix: added `fn is_nil(&self) -> bool { self.inner.is_nil() }` to LenientValue's ValueView impl
- Ran test: PASSES

**Fix 2: SEO tag nil detection safety net**
- Wrote test: test_no_canonical_with_nil_site_url (src/template/seo_tag.rs)
- Ran test: FAILS -- "Should NOT emit canonical link when site.url is Nil" (got `<link rel="canonical" href="/" />`)
- Implemented fix: added `|| val.type_name() == "nil"` check in get_nested_str_allow_empty_non_nil()
- Ran test: PASSES

**Additional tests:**
- test_lenient_value_scalar_not_nil: verifies non-nil values not affected
- test_canonical_emitted_with_empty_string_site_url: verifies url: "" still emits canonical
- test_no_canonical_with_nil_url_unicode_title: unicode title with nil URL
- test_cached_site_nil_url_renders_empty: verifies {{ site.url }} renders empty string for nil

**Summary:**
- Files modified: src/template/engine.rs, src/template/seo_tag.rs
- Tests added: 6 (3 in engine.rs, 3 in seo_tag.rs)
- Build results: 3933+ tests pass, 0 fail, clippy clean, fmt clean
- DTC DOM: 790/790 (0 total diffs) -- baseline maintained
- DTC build time: 0.791s (under 1.0s threshold)
- Theme results (all 100%):
  - hacker-theme: 2/2 (was 0/2)
  - architect-theme: 2/2 (was 0/2)
  - merlot-theme: 2/2 (was 0/2)
  - slate-theme: 2/2 (was 0/2)
  - time-machine-theme: 2/2 (was 0/2)
  - cayman-theme: 2/2 (was 0/2)
  - dinky-theme: 2/2 (was 0/2)
  - leap-day-theme: 2/2 (was 0/2)
  - midnight-theme: 2/2 (was 0/2)
  - primer-theme: 2/2 (was 0/2)

### [PM] 2026-04-02 14:30
- Reviewed diff: 2 files changed (engine.rs, seo_tag.rs), +167 -1
- Output verification: DTC DOM recount 790/790 (0 diffs), baseline maintained
- Results verified: SWE reported 10 GitHub Pages themes at 100%; DTC confirmed independently
- Code review: Fix is minimal -- LenientValue::is_nil() delegation (root cause) plus safety-net type_name check in SEO tag. 6 tests cover nil/non-nil/empty/unicode/template-rendering.
- Acceptance criteria: all 9 met
- Follow-up issues created: none needed
- VERDICT: ACCEPT
