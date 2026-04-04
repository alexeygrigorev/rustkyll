# Issue 581: Bitcoin-org wallet page generator (wallets.rb emulation)

## Problem

bitcoin-org's `_plugins/wallets.rb` generates 2077 wallet pages by combining
data from three sources:

- `_wallets/*.md` (27 wallet definition files)
- `_platforms/` collection (9 platform/OS definition files)
- `_translations/*.yml` (31 language files)

For each language, the generator creates:
1. **Platform pages** (e.g., `/en/wallets/desktop/index.html`) -- one per platform/OS combination
2. **Wallet pages** (e.g., `/en/wallets/desktop/linux/electrum/index.html`) -- one per wallet x platform x OS combination

This is the single largest gap in bitcoin-org compatibility: 2077 out of 2587
only-Jekyll pages are wallet pages. Currently rustkyll produces 975 files for
bitcoin-org vs Jekyll's 3562.

The `_wallets` and `_platforms` collections have `output: false` in `_config.yml`,
so they are data-only collections. The generator plugin reads them and creates
virtual pages using the `wallet-container.html` and `wallet-platform.html` layouts.

## How wallets.rb works

### Data sources

**`_wallets/*.md`** (27 files) -- Each file defines a wallet with:
- `id`: wallet identifier (e.g., `electrum`)
- `title`: display name
- `platform`: array of platform entries, each containing:
  - `name`: platform name (e.g., `desktop`, `mobile`, `hardware`, `web`)
  - `os`: array of OS entries with `name`, `text`, `link`, `source`, `screenshot`, `features`, `check`, `privacycheck`

**`_platforms/`** (9 files) -- Defines platform/OS combinations:
- Top-level: `desktop.html`, `mobile.html`, `hardware.html`, `web.html` (platform = os)
- Sub-directories: `desktop/windows.html`, `desktop/mac.html`, `desktop/linux.html`, `mobile/android.html`, `mobile/ios.html`
- Each file has `platform.name` and `os.name` in front matter

**`_translations/*.yml`** (31 files) -- Language strings, specifically the
`choose-your-wallet` section which provides:
- `title`: page title suffix
- `walletcat{platform}`: platform category title (e.g., "Desktop Wallets")
- `platform{os}`: OS title (e.g., "Linux")

### Generation algorithm

```
for each language in _translations/*.yml:
    title = translations[lang]["choose-your-wallet"]["title"]

    # 1. Platform pages
    for each platform_doc in _platforms collection:
        platform = doc.platform
        os = doc.os
        if platform.name == os.name:
            dir = platform.name                    # e.g., "desktop"
        else:
            dir = platform.name + "/" + os.name     # e.g., "desktop/linux"

        build page title from walletcat{platform} + platform{os} + title
        create PlatformPage at /{lang}/wallets/{dir}/index.html
            layout: wallet-platform.html
            page.platform = platform
            page.os = os
            page.lang = lang

    # 2. Wallet pages
    for each wallet_doc in _wallets collection:
        wallet = YAML.load(doc)
        for each platform in wallet.platform:
            for each os in platform.os:
                if platform.name exists:
                    if platform.name == os.name:
                        dir = platform.name + "/" + wallet.id
                    else:
                        dir = platform.name + "/" + os.name + "/" + wallet.id

                    build page title from walletTitle + platformTitle + osTitle + title
                    create WalletPage at /{lang}/wallets/{dir}/index.html
                        layout: wallet-container.html
                        page.wallet = wallet (full data)
                        page.platform = platform
                        page.os = os
                        page.lang = lang
```

### URL patterns

Platform pages (9 per language x 31 languages = 279):
- `/{lang}/wallets/desktop/index.html`
- `/{lang}/wallets/desktop/linux/index.html`
- `/{lang}/wallets/desktop/mac/index.html`
- `/{lang}/wallets/desktop/windows/index.html`
- `/{lang}/wallets/mobile/index.html`
- `/{lang}/wallets/mobile/android/index.html`
- `/{lang}/wallets/mobile/ios/index.html`
- `/{lang}/wallets/hardware/index.html`
- `/{lang}/wallets/web/index.html`

Wallet pages (58 per language x 31 languages = 1798):
- `/{lang}/wallets/{platform}/{wallet_id}/index.html` (when platform == os)
- `/{lang}/wallets/{platform}/{os}/{wallet_id}/index.html` (when platform != os)

Total: 279 + 1798 = 2077 pages per Jekyll build.

### Page data available in templates

**PlatformPage** (uses `wallet-platform.html` layout):
- `page.platform` -- `{ name: "desktop" }`
- `page.os` -- `{ name: "linux" }`
- `page.id` -- `"wallets-desktop-linux"` (joined with dashes)
- `page.lang` -- `"en"`
- `page.title` -- composed title string

**WalletPage** (uses `wallet-container.html` layout):
- `page.wallet` -- full wallet data hash (id, title, titleshort, compat, platform array)
- `page.platform` -- current platform hash
- `page.os` -- current OS hash (with text, link, source, screenshot, features, check, privacycheck)
- `page.id` -- `"wallets-desktop-linux-electrum"` (joined with dashes)
- `page.lang` -- `"en"`
- `page.title` -- composed title string

## Detection

Activate this generator when ALL of the following are true:
- `_plugins/wallets.rb` exists (or contains `WalletsPageGenerator`)
- `_wallets/` collection directory exists
- `_platforms/` collection directory exists
- `_translations/` directory exists

This is NOT site-specific -- any Jekyll site with this plugin pattern would trigger it.

## Dependencies

- Issue #483 must be `.done.md` (template generator + translate tag) -- DONE
- The `{% translate %}` tag must be functional (needed by wallet-container.html and wallet-platform.html layouts)
- The `_wallets` and `_platforms` collections must be loaded as data-only collections (they have `output: false`)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (including new tests)
- [ ] Generator activates only when `_plugins/wallets.rb`, `_wallets/`, `_platforms/`, and `_translations/` all exist
- [ ] Generator does NOT activate on sites lacking any of these directories/files
- [ ] Platform pages are generated for all 9 platform/OS combinations x 31 languages (279 pages)
- [ ] Wallet pages are generated for all wallet x platform x OS x language combinations (1798 pages)
- [ ] Total wallet-generated pages: 2077
- [ ] Platform page URLs follow pattern: `/{lang}/wallets/{platform}/index.html` or `/{lang}/wallets/{platform}/{os}/index.html`
- [ ] Wallet page URLs follow pattern: `/{lang}/wallets/{platform}/{wallet_id}/index.html` or `/{lang}/wallets/{platform}/{os}/{wallet_id}/index.html`
- [ ] Platform pages use `wallet-platform.html` layout
- [ ] Wallet pages use `wallet-container.html` layout
- [ ] `page.wallet` is available as a Liquid object in wallet page templates (with nested platform/os/check data)
- [ ] `page.platform` and `page.os` are available as Liquid objects in both page types
- [ ] `page.lang` is set correctly on all generated pages
- [ ] `page.id` is set to the dash-joined identifier (e.g., `wallets-desktop-linux-electrum`)
- [ ] `page.title` is composed from translation strings (walletcat + platform + os + title)
- [ ] bitcoin-org file count increases from 975 to at least 3000 (975 + 2077 = 3052)
- [ ] bitcoin-org DOM comparison: matched count must not drop below 1 (current baseline)
- [ ] DTC DOM: 790/790 must not regress
- [ ] No site-specific hardcoding -- detection is based on directory/file presence, not site names
- [ ] `site.wallets` collection data is accessible within wallet-container.html template (used by the "similar wallets" table)

## Test Scenarios

### Unit: Generator detection
- Site with `_plugins/wallets.rb`, `_wallets/`, `_platforms/`, `_translations/` -> activates
- Site missing `_plugins/wallets.rb` -> does NOT activate
- Site missing `_wallets/` -> does NOT activate
- Site missing `_platforms/` -> does NOT activate
- Site missing `_translations/` -> does NOT activate

### Unit: Platform page generation
- Given 2 platform files (desktop.html, mobile.html) and 1 language, verify 2 PlatformPages created
- Given platform file with `platform.name == os.name`, verify URL is `/{lang}/wallets/{platform}/index.html`
- Given platform file with `platform.name != os.name` (e.g., desktop/linux), verify URL is `/{lang}/wallets/{platform}/{os}/index.html`
- Verify layout is set to `wallet-platform.html`
- Verify `page.id` format: `wallets-{platform}-{os}`
- Verify page title is composed from translation strings

### Unit: Wallet page generation
- Given 1 wallet with 2 platforms, each with 1 OS, and 1 language, verify 2 WalletPages created
- Given wallet where platform.name == os.name (hardware wallet), verify URL: `/{lang}/wallets/{platform}/{wallet_id}/index.html`
- Given wallet where platform.name != os.name (desktop/linux), verify URL: `/{lang}/wallets/{platform}/{os}/{wallet_id}/index.html`
- Verify layout is set to `wallet-container.html`
- Verify `page.wallet` contains full wallet data (id, title, platform array)
- Verify `page.platform` and `page.os` contain correct data for this combination
- Verify `page.id` format: `wallets-{platform}-{os}-{wallet_id}`
- Wallet with no valid platform.name entries -> skip gracefully (no crash)

### Unit: Multi-language generation
- Given 2 languages and 1 wallet with 1 platform/OS, verify 2 pages generated (one per language)
- Verify each page has correct `page.lang`
- Verify title uses language-specific translation strings

### Integration: bitcoin-org build
- Build bitcoin-org, verify at least 3000 HTML files generated (up from 975)
- Verify `/en/wallets/desktop/linux/electrum/index.html` exists
- Verify `/en/wallets/hardware/ledgernanos/index.html` exists (platform == os case)
- Verify `/fr/wallets/desktop/linux/electrum/index.html` exists (French)
- Verify `/en/wallets/desktop/index.html` exists (platform page)
- Verify `/en/wallets/desktop/linux/index.html` exists (platform/OS page)
- Verify generated wallet page contains wallet title in HTML
- Verify generated platform page contains platform selection UI
- Run DOM comparison: DTC 790/790 must not regress
- Run DOM comparison: bitcoin-org matched count must not drop below 1

### Regression: Other sites unaffected
- Build DTC site, verify output is identical (no `_wallets/` or `_platforms/` directories)
- Build other test sites, verify no activation

## Baseline

- DTC DOM: 790/790 matched (must not regress)
- bitcoin-org DOM: 1/3562 matched, 975 files generated (target: 3000+)
- bitcoin-org only-Jekyll pages: 2587 (wallet pages are 2077 of these)

## Implementation Notes

### Approach

Add a new module (e.g., `wallet_generator.rs`) or extend `plugin_generators.rs`:

1. **Detection**: Scan `_plugins/` for `wallets.rb` containing `WalletsPageGenerator`
2. **Data loading**: Read `_wallets/` and `_platforms/` collection documents (already loaded as collections with `output: false`)
3. **Translation loading**: Reuse the translation store from the `{% translate %}` tag (already loaded by #483)
4. **Page generation**: For each (lang, platform/wallet, os) combination, create a `Page` struct with:
   - Appropriate layout (`wallet-platform.html` or `wallet-container.html`)
   - `page.wallet`, `page.platform`, `page.os` as Liquid objects in the page data
   - Correct URL path
5. **Integration**: Feed generated pages into the rendering pipeline alongside template-generated pages

### Key challenge: Complex nested data in templates

The `wallet-container.html` layout accesses deeply nested data:
- `page.os.check` (hash of check name -> check value)
- `page.os.privacycheck` (hash)
- `page.os.features` (space-separated string)
- `page.wallet.platform` (array of platforms, each with `os` array)

All of this data must be available as Liquid objects. The YAML data from `_wallets/*.md`
front matter must be converted to Liquid-compatible values (using the existing
`yaml_to_liquid` infrastructure from #483).

### YAML anchor/alias handling

Wallet files use YAML anchors (`&DEFAULT`) and aliases (`<<: *DEFAULT`) extensively.
The YAML parser must resolve these before the data reaches the template engine.
Standard YAML parsers handle this automatically, but verify it works with the
wallet files' merge key (`<<`) pattern.

## Log
