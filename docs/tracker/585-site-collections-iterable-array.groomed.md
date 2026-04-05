# Issue #585: Expose site.collections as iterable array with label, docs, output

## Problem

Jekyll exposes `site.collections` as an array of collection objects, each with properties
like `.label`, `.docs`, `.output`, `.directory`, etc. Rustkyll does not expose
`site.collections` at all -- it only sets `site.<collection_name>` (e.g., `site.portfolio`,
`site.talks`).

Templates that iterate over `site.collections` to render cross-collection archives get
empty output.

**Example from academicpages `_pages/collection-archive.html`:**
```liquid
{% for collection in site.collections %}
  {% unless collection.output == false or collection.label == "posts" %}
    <h2>{{ collection.label }}</h2>
  {% endunless %}
  {% for post in collection.docs %}
    {% include archive-single.html %}
  {% endfor %}
{% endfor %}
```

**Expected:** Lists all collection items grouped by collection (portfolio, publications, talks, teaching)
**Actual:** Empty -- `site.collections` is nil so the loop never executes

## Affected Sites

- **academicpages**: collection-archive page (18 missing elements), page-archive (14),
  sitemap, and potentially other archive pages
- **minimal-mistakes**: collection-archive and sitemap pages (similar templates)
- **just-the-docs**: footer.html uses `site.collections`

## Jekyll's site.collections Structure

In Jekyll, `site.collections` returns an array where each element is a collection object:
```ruby
# Each collection has:
collection.label        # => "portfolio"
collection.docs         # => array of documents in the collection
collection.output       # => true/false (whether collection pages are generated)
collection.directory    # => absolute path to collection directory
collection.relative_directory # => relative path
collection.files        # => static files in the collection
```

When iterated with `{% for collection in site.collections %}`, each `collection` behaves
like a two-element array `[label, collection_object]` (due to Ruby Hash iteration), so
`collection.label` and `collection[0]` both return the label, and `collection.docs` and
`collection[1].docs` both work.

## Acceptance Criteria

- [ ] `site.collections` is an iterable array in Liquid templates
- [ ] Each element has `.label` returning the collection name (e.g., "portfolio")
- [ ] Each element has `.docs` returning an array of collection documents
- [ ] Each element has `.output` returning the boolean output setting
- [ ] `{% for collection in site.collections %}` iterates over all collections
- [ ] The `posts` collection is included in `site.collections` (Jekyll includes it)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM match count must not drop below 789/790

## Test Scenarios

### Unit: site.collections structure
- Create a config with 2 custom collections (portfolio, talks) + posts
- Verify `site.collections` is an array with 3 elements
- Verify each element has `.label`, `.docs`, `.output` properties
- Verify `.docs` contains the collection items

### Unit: collection.label
- Verify `{% for c in site.collections %}{{ c.label }}{% endfor %}` outputs collection names

### Unit: collection.docs iteration
- Verify `{% for doc in collection.docs %}{{ doc.title }}{% endfor %}` lists document titles

### Integration: academicpages collection-archive
- Build academicpages site
- Verify collection-archive/index.html contains portfolio, publications, talks, teaching sections
- Verify DOM match count improves from 10/45

## Dependencies

None.

## DOM Baseline

- DTC: 789/790 matched
- academicpages: 10/45 matched, 298 total diffs

## Log

### [PM] 2026-04-02 10:00
- Created from analysis of academicpages DOM diffs
- collection-archive page entirely empty due to missing site.collections
- Also affects minimal-mistakes and just-the-docs
