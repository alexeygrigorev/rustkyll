# Issue 483: Bitcoin-org plugin generator emulation

## Problem

bitcoin-org uses 5 custom Ruby Generator plugins that programmatically create pages.
Rustkyll currently produces 142 HTML files vs Jekyll's 3562 -- a gap of ~3420 pages,
almost entirely caused by missing generator support. The generators are:

| Generator | Source | Output pattern | Pages |
|-----------|--------|---------------|-------|
| templates.rb | `_translations/*.yml` x `_templates/*.html` | `/{lang}/{translated-url}.html` | ~900 |
| wallets.rb | `_wallets/*.md` x `_platforms/` x translations | `/{lang}/wallets/{platform}/{os}/{wallet}/index.html` | ~2077 |
| releases.rb | `_releases/*.md` | `/en/release/v{version}.html` | ~63 |
| alerts.rb | `_alerts/*.{html,md}` | `/en/alert/{file}` | ~15 |
| redirects.rb | `redirects:` in `_config.yml` | various | ~100 |

Additionally, all templates use a custom `{% translate id %}` Liquid tag that looks up
localized strings from `_translations/{lang}.yml`. Without this tag, even generated
pages produce empty/broken output.

## Revised Scope: Phase 1 -- Template Generator + Translate Tag

The original issue proposed a fully generic YAML-based generator schema. After
investigating the actual site, a more practical approach is to implement the two
features that unlock the most pages:

### 1. Template translation generator (templates.rb emulation)

This is the core pattern: for each language file in `_translations/`, for each
template in `_templates/`, generate a page at the translated URL.

**How the Ruby plugin works:**
1. Load each `_translations/{lang}.yml` -- the file contains `{lang}: { url: { template-id: translated-slug }, ... }`
2. For each language, for each template file in `_templates/`:
   - Look up the template's ID (filename without extension) in `translations[lang][url]`
   - If a URL mapping exists, create a page at `/{lang}/{translated-url}.html`
   - Set `page.lang` to the language code
   - The template content + front matter (including layout) are read from the template file
3. Additionally, always generate `/{lang}/index.html` from `_templates/index.html`

**Detection:** If a site has both `_translations/` directory and `_templates/` directory,
and `_plugins/templates.rb` exists (or contains `TranslatePageGenerator`), activate this
generator.

**Implementation approach:**
- Add a new function in `plugin_generators.rs` (or a new module `bitcoin_generators.rs`)
- At build time, after loading collections but before rendering:
  1. Scan `_translations/*.yml`, parse each YAML file
  2. Scan `_templates/*.html`, read their front matter + content
  3. For each (lang, template) pair, create a `Page` struct with:
     - `content` = template content
     - `front_matter` = template's front matter + `lang: {lang_code}`
     - `url` = `/{lang}/{translated_url}` (looked up from translation data)
     - `source_path` = `_templates/{file}`
  4. Feed these virtual pages through the normal `generate_pages_cached` pipeline

### 2. Custom `{% translate %}` Liquid tag

Every template and most layouts/includes use `{% translate id %}` (55+ files).
Without this, generated pages render with empty strings everywhere.

**How the Ruby tag works:**
1. Parse arguments: `{% translate id [category] [lang] %}`
   - `id` = translation key name
   - `category` = defaults to `page.id` (the template filename without extension)
   - `lang` = defaults to `page.lang`
2. Look up `translations[lang][category][id]`
3. If empty, fall back to English: `translations["en"][category][id]`
4. Process the result through Liquid (translations can contain Liquid expressions)
5. Replace URL references: `#template-id#` becomes `/{lang}/{translated-url}`
6. Replace anchor references: `[page.anchor-key]` becomes the translated anchor text

**Implementation approach:**
- Register a custom Liquid tag `translate` in the template engine
- Load all translation data once at site build time, store in `site.config["loc"]`
- The tag reads `page.lang` and `page.id` from context, looks up the string
- URL replacement: for each key in `translations[lang]["url"]`, replace `#key#` with `/{lang}/{value}`
- Anchor replacement: for each page/key in `translations[lang]["anchor"]`, replace `[page.key]` with value

## Out of Scope (follow-up issues)

These are explicitly deferred and should be tracked separately:

- **Wallet page generator** (wallets.rb) -- complex nested logic with platform/os/wallet combinations, ~2077 pages. Needs its own issue.
- **Release page generator** (releases.rb) -- simpler directory scan, ~63 pages. Needs its own issue.
- **Alert page generator** (alerts.rb) -- directory scan with date parsing, ~15 pages. Needs its own issue.
- **Redirect page generator** (redirects.rb) -- reads `redirects:` from config, ~100 pages. Needs its own issue.
- **Site data generators** (events.rb, contributors.rb) -- populate site variables from external APIs, no page generation. Low priority.
- **Generic YAML-based generator config** -- the original proposal. Deferred until we understand whether the bitcoin-org-specific patterns generalize to other sites.

## Dependencies

- No other issues need to be `.done.md` first
- This issue does NOT depend on any existing generator work (author/tag generators in `plugin_generators.rs` are for Jasper2-style sites, orthogonal to this)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (including new tests below)
- [ ] When building bitcoin-org, rustkyll generates translated template pages for all 31 languages x 29 templates (where URL mappings exist)
- [ ] Generated pages are placed at the correct translated URL paths (e.g., `/fr/a-propos-de-nous.html`, `/de/ueber-uns.html`)
- [ ] Each generated page has `page.lang` set to the correct language code
- [ ] Each generated page has `page.id` set to the template ID (filename stem)
- [ ] The `{% translate id %}` Liquid tag resolves translation strings from `_translations/` YAML files
- [ ] The `{% translate id %}` tag falls back to English when a translation is missing
- [ ] URL references in translated strings (`#template-id#`) are replaced with `/{lang}/{translated-url}`
- [ ] Anchor references in translated strings (`[page.anchor-key]`) are replaced with translated anchor text
- [ ] The `{% translate id category %}` two-argument form works (explicit category override)
- [ ] The `{% translate id category lang %}` three-argument form works (explicit lang override)
- [ ] Dynamic variables in translate arguments work: `{% translate menu-{{id}} %}`
- [ ] bitcoin-org file count increases from 142 to at least 1000 (template pages add ~900)
- [ ] DTC DOM match count does not drop below 596 (current baseline)
- [ ] bitcoin-org DOM match count does not drop below 1 (current baseline)
- [ ] No site-specific hardcoding -- detection is based on directory/file presence, not site names
- [ ] The translate tag handles missing translation keys gracefully (returns empty string, does not crash)
- [ ] The translate tag processes Liquid expressions inside translated strings (translations can contain `{{ }}` and `{% %}`)

## Test Scenarios

### Unit: Translation YAML loading
- Load a translation YAML file with nested structure (`lang: { url: { ... }, page-id: { key: value } }`)
- Verify URL mappings are extracted correctly
- Verify translation strings are accessible by (lang, category, id) triple
- Verify missing language falls back to empty/English

### Unit: `{% translate %}` tag parsing
- Parse `{% translate title %}` -- single argument, category from page.id, lang from page.lang
- Parse `{% translate button-ok layout %}` -- two arguments, explicit category
- Parse `{% translate greeting layout en %}` -- three arguments, explicit lang
- Parse `{% translate menu-{{page.section}} %}` -- dynamic variable interpolation

### Unit: `{% translate %}` tag rendering
- Render with existing translation key, verify correct string returned
- Render with missing translation key in target lang, verify English fallback
- Render with missing key in both target and English, verify empty string (no crash)
- Render with translation string containing `#url-id#`, verify URL replacement to `/{lang}/{slug}`
- Render with translation string containing `[page.anchor]`, verify anchor replacement
- Render with translation string containing Liquid (`{{ site.url }}`), verify Liquid is processed
- Render with Unicode content (Arabic, Chinese, Hebrew), verify correct output

### Unit: Template page generation
- Given 1 translation file and 2 templates, verify 2 pages generated
- Verify output paths match the URL mappings from the translation file
- Verify `page.lang` is set on each generated page
- Verify `page.id` is set to the template filename stem
- Verify template front matter (especially `layout`) is preserved on generated pages
- Verify `index.html` template always generates `/{lang}/index.html`
- Verify templates with no URL mapping in a language are skipped (not crash)

### Integration: bitcoin-org build
- Build bitcoin-org with rustkyll, verify at least 1000 HTML files generated (up from 142)
- Verify `/en/about-us.html` exists and contains content from `_templates/about-us.html`
- Verify `/fr/a-propos-de-nous.html` exists (French translated URL)
- Verify `/de/` directory contains German-translated pages
- Verify `page.lang` is correctly set by checking that language-specific content appears
- Run DOM comparison: bitcoin-org DOM match count must not drop below 1
- Run DOM comparison: DTC DOM match count must not drop below 596

### Regression: DTC site unaffected
- Build DTC site, verify output is identical to before this change
- DTC has no `_translations/` or `_templates/` directories, so generator should not activate

## Baseline

- DTC DOM: 596/790 matched (must not regress)
- bitcoin-org DOM: 1/127 matched, 142/3562 files (must not regress)
- bitcoin-org file count: 142 HTML files (target: 1000+)

## Implementation Notes

### Translation data structure

Each `_translations/{lang}.yml` has this structure:
```yaml
en:
  url:
    about-us: about-us
    buy: buy
    community: community
    ...
  anchor:
    community:
      non-profit: non-profit-organisation
    vocabulary:
      wallet: wallet
      ...
  about-us:
    title: "About bitcoin.org"
    pagetitle: "About bitcoin.org"
    ...
  buy:
    title: "Buy Bitcoin"
    ...
```

The `url` section maps template IDs to localized URL slugs.
The `anchor` section maps page.anchor-key pairs to localized anchor text.
Other sections are keyed by template ID (= `page.id`) and contain the translation strings.

### Template page generation algorithm

```
for each _translations/{lang}.yml:
    translations = parse YAML, extract lang subtree
    for each _templates/{file}.html:
        template_id = filename without extension
        translated_url = translations["url"][template_id]
        if translated_url is empty or missing: skip
        if translated_url ends with "/": append "index"
        output_path = "/{lang}/{translated_url}.html"
        create Page {
            content: file content (after front matter),
            front_matter: file front matter + { lang: lang, id: template_id },
            url: output_path,
            source_path: "_templates/{file}"
        }
    also always create /{lang}/index.html from _templates/index.html
```

### Translate tag algorithm

```
{% translate id [category] [lang] %}

1. id = first arg (may contain {{ }} for interpolation)
2. category = second arg or page.id
3. lang = third arg or page.lang (default "en" if empty)
4. text = translations[lang][category][id]
5. if text is empty and lang != "en":
     text = translations["en"][category][id]
6. text = Liquid::Template.parse(text).render(context)
7. for each (key, value) in translations[lang]["url"]:
     text.replace("#key#", "/{lang}/{value}")
8. for each (page, anchors) in translations[lang]["anchor"]:
     for each (key, value) in anchors:
       text.replace("[page.key]", "{value}")
9. return text
```

## Log

### [PM] 2026-04-02 grooming
- Investigated bitcoin-org: 5 generator plugins create ~3420 pages
- The biggest generator (templates.rb) creates ~900 pages via 29 templates x 31 languages
- All templates depend on custom `{% translate %}` Liquid tag (55+ files use it)
- A fully generic YAML schema is over-engineered for the actual patterns
- Scoped Phase 1 to template generator + translate tag (unlocks ~900 pages, ~25% of gap)
- Wallet/release/alert/redirect generators deferred to follow-up issues
- DTC baseline: 596/790 DOM match
- bitcoin-org baseline: 1/127 DOM match, 142 files
