# Issue 483: YAML-based page generator for custom site-specific generators

## Problem

Sites like bitcoin-org use custom Ruby Generator plugins that create
pages programmatically (e.g., 29 templates × 30 languages = 870 pages).
We can't run Ruby plugins, but the pattern is simple and repeatable:
"for each item in data source, create a page from a template."

## Proposed Design

A `_generators.yml` (or `generators:` key in `_config.yml`) that lets
sites declare page generation rules without Ruby:

```yaml
generators:
  # bitcoin-org style: template × translations
  - name: translations
    for_each: _translations/*.yml   # iterate over data files
    variable: lang                   # expose as {{ lang }}
    template: _templates/{item.template}  # template file
    output: "{lang.id}/{item.url}"   # output path pattern
    data_key: translations           # key within each YAML file

  # jekyll-archives style: pages per tag/category
  - name: tag_pages
    for_each: site.tags              # iterate over site.tags
    variable: tag
    template: _layouts/tag.html
    output: "tags/{tag.name}/index.html"

  # Simple data-driven pages
  - name: wallet_pages
    for_each: _data/wallets.yml
    variable: wallet
    template: _templates/wallet.html
    output: "wallets/{wallet.id}/index.html"
```

## How It Works

1. At build time, rustkyll reads `_generators.yml` (or config key)
2. For each generator rule:
   - Resolve the data source (glob files, site collection, data file)
   - For each item, create a virtual Page with:
     - Content from the template file
     - Frontmatter merged with the item data
     - Output path from the pattern
3. Virtual pages go through normal Liquid + layout rendering
4. No Ruby needed — pure data-driven generation

## bitcoin-org Example

Current Ruby plugin reads `_translations/*.yml` and `_templates/*.html`:
```ruby
Dir.foreach('_translations') do |file|
  translations = YAML.load_file(file)
  Dir.foreach('_templates') do |template|
    site.pages << TranslatePage.new(site, lang, template)
  end
end
```

Equivalent YAML config:
```yaml
generators:
  - name: translated_pages
    for_each: _translations/*.yml
    variable: translation
    nested_for_each: _templates/*.html
    nested_variable: template
    output: "{translation.id}/{template.url}"
```

## Scope

1. Design the YAML schema
2. Implement the generator engine in Rust
3. Test with bitcoin-org as the reference site
4. Document the schema

## Priority

Medium — this unblocks bitcoin-org (3420 missing pages) and provides
a generic solution for any site with custom generators.

## Baseline

DTC 790/790. Must not regress.
