# Issue 514: SEO tag does not extract path from hash-type page.image frontmatter

## Problem

When `page.image` is a hash (e.g. Chirpy's `image: {path: ..., lqip: ..., alt: ...}`),
the SEO tag concatenates all hash values into `og:image` content instead of extracting
just the `path` field.

### Example

Frontmatter:
```yaml
image:
  path: /commons/devices-mockup.png
  lqip: data:image/webp;base64,UklGR...
  alt: Responsive rendering of Chirpy theme on multiple devices.
```

**Jekyll** (correct):
```html
<meta property="og:image" content="/commons/devices-mockup.png" />
<meta property="og:image:alt" content="Responsive rendering of Chirpy theme on multiple devices." />
```

**Rustkyll** (broken):
```html
<meta property="og:image" content="altResponsive rendering...lqipdata:image/webp;base64,...path/commons/devices-mockup.png" />
```

No `og:image:alt` tag is emitted, and the og:image content is garbage.

### Root Cause

`src/template/seo_tag.rs` line 327 reads `page.image` via `get_nested_str()`, which
calls `val.to_kstr()` on the hash, concatenating all values. Jekyll's `jekyll-seo-tag`
plugin checks whether `page.image` is a Hash and extracts `.path` for the URL and
`.alt` for the alt text.

### Affected Pages

- chirpy: `posts/text-and-typography/index.html` (~10 diffs from og:image cascade)
- chirpy: `index.html` (twitter:image also affected)
- Any site using hash-type image frontmatter (common in Chirpy, al-folio)

## Fix

In `src/template/seo_tag.rs`, change the `page_image` extraction to:

1. Try `page.image.path` first (hash case)
2. Fall back to `page.image` as a string (simple string case)
3. Also extract `page.image.alt` if present, and emit `og:image:alt` meta tag

Jekyll's seo-tag also supports `page.image.facebook` and `page.image.twitter` overrides,
but those are low priority. The `path` + `alt` extraction is the critical fix.

## Dependencies

None.

## Baseline

- DTC: 790/790 (must not regress)
- Chirpy: 12/17 (should not regress; may improve text-and-typography)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] When `page.image` is a hash with `path` key, `og:image` uses the path value
- [ ] When `page.image` is a hash with `alt` key, `og:image:alt` meta tag is emitted
- [ ] When `page.image` is a plain string, behavior is unchanged
- [ ] `twitter:image` also correctly extracts from hash-type image
- [ ] DTC DOM baseline remains at 790/790
- [ ] Chirpy DOM match count does not drop below 12/17

## Test Scenarios

### Unit: hash-type page.image extraction
- page.image = `{path: "/img/test.png", alt: "Test image"}` -- verify og:image = "/img/test.png"
- page.image = `{path: "/img/test.png", lqip: "data:...", alt: "Test"}` -- verify lqip is not in og:image
- page.image = "/img/test.png" (string) -- verify og:image = "/img/test.png" (unchanged behavior)
- page.image = `{path: "/img/test.png", alt: "Alt text"}` -- verify og:image:alt = "Alt text"
- page.image = `{path: "/img/test.png"}` (no alt key) -- verify no og:image:alt emitted

### Integration: chirpy site
- Build chirpy, check text-and-typography page has correct og:image with just the path
- Verify og:image:alt contains the alt text, not lqip data
