# Issue 324: opensource-guide template rendering and SEO tag structural diffs

## Problem

The opensource-guide site has 4404 DOM diffs across 365 pages. Approximately 1760
of these (~40%) come from 5 specific structural problems that are independent of
markdown content rendering. Fixing these would flip many pages from "diff" to
"match" and dramatically reduce the total diff count.

## Specific Issues

### 1. Missing language dropdown in nav (365 diffs)

The nav template (`_includes/nav.html` line 17) has:
```liquid
{% if page.lang and site.data.locales.size > 1 %}
```

This condition evaluates to false in rustkyll, so the entire `<li>` with the
language `<select>` dropdown is missing from the output. The likely cause is that
`site.data.locales` (loaded from `_data/locales/` directory of YAML files) does
not support the `.size` property in liquid-rust, or the mapping/hash `.size` is
returning 0 instead of the number of keys (29 locale files).

**Expected:** The third `<li>` in the nav `<ul>` should contain a language
dropdown `<select>` with options for all 29 locales.

**Actual:** Only 2 `<li>` elements (About and Contribute) appear in the nav.

### 2. Missing hreflang `<link>` elements (339 diffs)

The head template (`_includes/head.html` lines 11-24) has:
```liquid
{% if page.lang and page.untranslated != true and site.data.locales.size > 1 %}
{% assign locales = site.data.locales | sort %}
{% for locale in locales %}
```

Same root cause as issue 1 -- `site.data.locales.size > 1` is false. Each page
should have ~29 `<link rel="alternate" hreflang="...">` elements in `<head>`.

### 3. Extra `>` character after `</ol>` in TOC (328 diffs)

The jekyll-toc.html include's final block contains:
```liquid
{% capture jekyll_toc %}<{{ listModifier }}{{ rootAttributes }}>{{ nodes | shift | join: '>' }}>{% endcapture %}
```

The `shift` filter (removes first element of array) combined with `join: '>'`
and a trailing `>` produces `</ol>>` instead of `</ol>` in rustkyll output.
Either the `shift` filter is not implemented/behaving differently, or the join
behavior differs.

**Expected:** `</ol>` with proper whitespace
**Actual:** `</ol>>`

### 4. Missing `article:publisher` meta tag (affects 365 pages, causes ~850 cascade diffs)

Jekyll's SEO tag plugin reads `site.facebook.publisher` from `_config.yml` and
outputs:
```html
<meta property="article:publisher" content="https://www.facebook.com/GitHub/" />
```

The opensource-guide config has:
```yaml
facebook:
  publisher: https://www.facebook.com/GitHub/
```

Rustkyll's `seo_tag.rs` does not implement this. The missing meta tag shifts all
subsequent elements in `<head>` by one position, causing `child[13]:
tag_name_differs` (363 diffs) and `script: missing_element` (365 diffs) as
cascade effects.

### 5. SEO tag JSON-LD formatting (minor)

Jekyll outputs JSON-LD on a single line with `</script>` on the same line.
Rustkyll splits the closing `</script>` onto a new line. This is a minor
formatting difference but contributes to the tag position shifts.

Also, rustkyll's JSON-LD is missing `dateModified` and `mainEntityOfPage` fields
that Jekyll's SEO tag includes. These may already be filtered as acceptable diffs.

## Impact Summary

| Pattern | Count | Root Cause |
|---------|-------|------------|
| Missing nav `<li>` | 365 | `site.data.locales.size` not working |
| Missing hreflang `<link>` | 339 | Same as above |
| Extra `>` in TOC | 328 | `shift` filter or join behavior |
| `child[13]` tag_name_differs | 363 | Missing article:publisher cascade |
| `script` missing_element | 365 | Missing article:publisher cascade |
| **Total** | **~1760** | |

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `site.data.locales.size` returns the correct count (29) when locales directory has 29 YAML files
- [ ] `site.data.locales | sort` returns a sorted iterable of (key, value) pairs
- [ ] `locale[0]` returns the key (locale name) when iterating over the sorted mapping
- [ ] `locale[1][lang].locale_name` resolves correctly to the nested locale name
- [ ] Nav contains 3 `<li>` elements including the language dropdown
- [ ] hreflang `<link>` elements are present in `<head>` for all locales
- [ ] `</ol>` in TOC output does not have a trailing `>` character
- [ ] `article:publisher` meta tag is output when `site.facebook.publisher` is configured
- [ ] JSON-LD `</script>` is on the same line as the JSON content (no extra newline)
- [ ] `cargo test` passes with new tests for each fix
- [ ] Building opensource-guide and running DOM comparison shows significant diff reduction

## Test Scenarios

### Unit: Liquid `.size` on data mappings
- Create a data directory with 3 YAML files, verify `site.data.dirname.size` returns 3
- Verify `{% if site.data.dirname.size > 1 %}` evaluates to true
- Verify `site.data.dirname | sort` produces sorted key-value pairs
- Verify `item[0]` and `item[1]` access works on sorted mapping entries

### Unit: `shift` filter
- Verify `{{ array | shift }}` removes the first element
- Verify `{{ array | shift | join: '>' }}` produces correct output
- If `shift` is not a standard Liquid filter, verify it's registered as a custom filter

### Unit: SEO tag article:publisher
- Render SEO tag with `site.facebook.publisher` set, verify `article:publisher` meta tag is output
- Render SEO tag without `site.facebook.publisher`, verify no `article:publisher` tag
- Verify the meta tag appears in correct position (after twitter:site, before JSON-LD script)

### Unit: JSON-LD formatting
- Verify JSON-LD and `</script>` are on the same line with no intervening newline

### Integration: opensource-guide site build
- Build the full site and run DOM comparison
- Verify the nav has 3 `<li>` elements on any page
- Verify hreflang links are present in `<head>`
- Verify no `</ol>>` appears anywhere in the output
- Verify `article:publisher` meta tag is present

## Output Verification

- Build opensource-guide with rustkyll
- Run DOM comparison: expect total diffs to drop from 4404 to approximately 2600 or fewer
- Check that matched pages increase from 23 to 50+ (pages where these structural issues were the only diffs)
- Verify `grep -r '</ol>>' websites/opensource-guide/_site_rustkyll/` returns zero matches
- Verify `grep -c 'hreflang' websites/opensource-guide/_site_rustkyll/best-practices/index.html` returns ~29+

## Dependencies

None -- these are independent fixes to the template engine, filters, and SEO tag plugin.

## Log

### [SWE] 2026-03-23

**Fix 1: `.size` on Liquid data mappings (LenientValue objects)**

- Root cause: `LenientValue::get()` returns `Some(&self.nil)` for ALL missing keys, including "size". This prevents `augmented_get` in liquid-core's `find.rs` from falling through to `obj.size()` which computes the correct key count.
- Wrote 3 tests: `test_size_on_nested_object_via_lenient_render`, `test_size_on_nested_object_in_condition`, `test_size_on_object_with_unicode_keys`
- All 3 tests FAIL as expected (got "" instead of "3", condition evaluates to false)
- Fixed `LenientValue::get()` in `src/template/engine.rs:227`: return `None` for "size", "first", "last" when not actual keys, so `augmented_get` can compute built-in values
- All 3 tests PASS after fix

**Fix 2: `article:publisher` meta tag**

- Root cause: `seo_tag.rs` never reads `site.facebook.publisher` or outputs the `article:publisher` meta tag
- Wrote 3 tests: `test_article_publisher_meta_tag_present`, `test_article_publisher_meta_tag_absent_without_config`, `test_article_publisher_meta_tag_unicode_url`
- 2 of 3 tests FAIL as expected (present and unicode); absent test passes
- Added `facebook_publisher` extraction and `article:publisher` meta tag output after `article:published_time`
- All 3 tests PASS after fix

**Build results:**
- 2619 tests total (2611 pass from my changes + 6 pre-existing failures from other SWE on issue 323 + 2 ignored)
- Clippy: pre-existing dead-code error in `layout.rs` from other SWE; my files are clean
- Fmt: my files pass; pre-existing formatting issues in other SWE's files

**Files modified:**
- `src/template/engine.rs` -- Fixed `LenientValue::get()` to not shadow built-in `.size`/`.first`/`.last` properties; added 3 tests
- `src/template/seo_tag.rs` -- Added `article:publisher` meta tag support; added 3 tests
