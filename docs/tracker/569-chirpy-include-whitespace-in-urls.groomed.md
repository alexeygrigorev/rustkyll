# Issue 569: Chirpy include output leaks whitespace into URLs and attributes

## Problem

Chirpy's `_includes/media-url.html` include produces URLs with embedded newlines and spaces in rustkyll output, while Jekyll produces clean URLs. This affects image paths, og:image meta tags, and link hrefs across multiple chirpy pages.

### Concrete example

Chirpy's `media-url.html` include template uses a mix of whitespace-controlled (`{%- -%}`) and non-controlled (`{% %}`) Liquid tags:

```liquid
{%- endcomment -%}

{% assign url = include.src %}

{%- if url -%}
  {% unless url contains ':' %}
    ...
    {% assign url = site.baseurl | append: url %}
    ...
  {% endunless %}
{%- endif -%}

{{- url -}}
```

**Jekyll output:** `content="/commons/devices-mockup.png"`

**Rustkyll output:** `content="\n\n    \n\n    \n      \n        \n      \n    \n  /commons/devices-mockup.png"`

The whitespace from `{% unless %}`, `{% if %}`, `{% endif %}`, `{% endunless %}` lines (which do NOT use dash whitespace control) is leaking into the include's rendered output, prepended to the URL.

## Root Cause

In Jekyll's Liquid engine, when an include template renders, whitespace from non-dash control flow tags (`{% if %}`, `{% unless %}`, etc.) IS part of the output. The `{{- url -}}` tag at the end strips whitespace immediately before and after itself. But in Jekyll, the whitespace between `{%- endif -%}` (line 35) and `{{- url -}}` (line 37) is just one blank line, which `{{-` strips.

In rustkyll, the whitespace from INSIDE the if/unless blocks (lines 16-34) appears to be accumulating in the output and not being properly stripped by the `{%- endif -%}` and `{{- url -}}` tags. This suggests the whitespace control (`{%-` / `-%}`) is not stripping whitespace across include template boundaries correctly.

## Affected Pages (chirpy)

- `posts/text-and-typography/index.html` -- 43 differences (og:image, img src/href attributes)
- `posts/getting-started/index.html` -- 16 differences (image paths)
- `index.html` -- 14 differences (preview image data-src)
- Other pages with images that use the media-url include

## Scope

- Debug and fix whitespace handling in Liquid include rendering for the chirpy media-url.html pattern
- The fix must correctly handle the interaction between `{%- -%}` (dash) and `{% %}` (non-dash) tags within includes
- Verify that `{{- -}}` output tags properly strip adjacent whitespace in include context

## Baseline

- DTC: 789/790 matched (163 total diffs). Must not regress.
- Chirpy: 12/17 matched (77 total diffs). Must improve.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new ones
- [ ] Chirpy `posts/text-and-typography/index.html` og:image content attribute equals `/commons/devices-mockup.png` (no embedded whitespace)
- [ ] Chirpy `posts/getting-started/index.html` image paths have no embedded whitespace
- [ ] DTC DOM match count does not drop below 789/790
- [ ] Chirpy DOM total diffs decrease from current 77

## Test Scenarios

### Unit: Include whitespace control with mixed dash/non-dash tags
- Render an include template with `{%- if -%}` ... `{% assign %}` ... `{%- endif -%}` ... `{{- var -}}` pattern
- Verify the output contains only the variable value with no leading whitespace
- Test with nested `{% unless %}` inside `{%- if -%}` blocks

### Unit: Whitespace stripping at include boundaries
- Render `{% include media-url.html src="/test.png" %}` with a minimal media-url-like template
- Verify the include output is exactly `/test.png` with no surrounding whitespace

### Integration: Chirpy image URL rendering
- Build chirpy site, extract og:image meta tag from text-and-typography page
- Verify the content attribute value starts with `/` (no whitespace prefix)

## Dependencies

None. Independent of issue 547 (capture whitespace) though both affect chirpy.
