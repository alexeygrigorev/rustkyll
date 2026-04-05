# Issue #586: Slugify drops combining marks and emoji from heading IDs

## Problem

The `slugify()` function in `src/kramdown.rs` (used for generating heading IDs) filters
characters using `ch.is_alphanumeric()`, which excludes:

1. **Unicode combining marks** (category Mn/Mc) like viramas, vowel signs, and nuktas
2. **Emoji characters** (category So) like hearts, stars, etc.

Ruby's `tr!('^\\w -', '')` operates on bytes and keeps ALL bytes >= 128 (non-ASCII),
regardless of Unicode category. This means Ruby preserves combining marks, emoji, and
any other non-ASCII character in heading slugs.

**Bengali example:**
- Heading text: `রক্ষণাবেক্ষণকারী বলতে কী বোঝায়?`
- Jekyll ID: `রক্ষণাবেক্ষণকারী-বলতে-কী-বোঝায়` (preserves virama U+09CD in conjuncts)
- Rustkyll ID: `রকষণাবেকষণকারী-বলতে-কী-বোঝায` (strips virama, breaking conjuncts)

**Emoji example:**
- Heading text: `Community is the ️ of open source`
- Jekyll ID: `community-is-the-️-of-open-source` (preserves heart emoji)
- Rustkyll ID: `community-is-the--of-open-source` (strips emoji, leaves double hyphen)

## Affected Sites

- **opensource-guide**: ~140+ pages affected across Bengali, Hindi, Arabic, and other
  language translations. The heading IDs don't match, and TOC links point to wrong anchors.
  Also affects ~20 pages across many languages where emoji in headings is stripped.
- Any site with non-Latin headings that contain combining marks

## Root Cause

In `src/kramdown.rs` line 5832-5835, the `slugify()` function:
```rust
if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == ' ' || ch == '\t' {
    slug.push(ch);
}
```

This should instead keep all non-ASCII characters (matching Ruby's byte-level `tr!`):
```rust
if !ch.is_ascii() || ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == ' ' || ch == '\t' {
    slug.push(ch);
}
```

The same fix may be needed in the kramdown_parser's `generate_header_id()` function
(line 170-173 in `src/kramdown_parser/html.rs`), but that function is documented as
`basic_generate_id` which is ASCII-only, so it may be correct as-is. The distinction:
- `slugify()` = GFM/default kramdown: keeps non-ASCII
- `basic_generate_id()` = kramdown base parser for markdown="1" blocks: ASCII-only

## Acceptance Criteria

- [ ] Bengali virama (U+09CD) preserved in heading IDs: `রক্ষণাবেক্ষণকারী` stays intact
- [ ] Hindi virama (U+094D) preserved: `प्रक्रिया` stays intact (not `परकरिया`)
- [ ] Devanagari vowel signs preserved: `स्रोत` stays intact (not `सरोत`)
- [ ] Emoji preserved in heading IDs: `community-is-the-️-of-open-source`
- [ ] ASCII-only behavior unchanged: `hello-world` still works
- [ ] The `basic_generate_id()` function is NOT changed (it's intentionally ASCII-only)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM match count must not drop below 789/790

## Test Scenarios

### Unit: Combining marks preserved in slugify
- `slugify("রক্ষণাবেক্ষণকারী বলতে কী বোঝায়")` => `রক্ষণাবেক্ষণকারী-বলতে-কী-বোঝায়`
- `slugify("प्रक्रिया नथিভুক্ত করা")` => `প্রক্রিয়া-নথিভুক্ত-করা` (with correct conjuncts)
- `slugify("खुले स्रोत का क्या")` => `खुले-स्रोत-का-क्या`

### Unit: Emoji preserved in slugify
- `slugify("Community is the ️ of open source")` => `community-is-the-️-of-open-source`
- `slugify("общността е ️ на отворен код")` => `общността-е-️-на-отворен-код`

### Unit: ASCII behavior unchanged
- `slugify("Hello World")` => `hello-world`
- `slugify("Hello, World!")` => `hello-world` (punctuation still stripped)
- `basic_generate_id("Hello World")` => `hello-world` (unchanged)

### Unit: Non-alphanumeric ASCII still stripped
- `slugify("hello@world#test")` => `helloworldtest` (@ and # stripped)

### Integration: opensource-guide heading IDs
- Build opensource-guide site
- Verify Bengali heading IDs match Jekyll output
- Verify emoji heading IDs match Jekyll output
- DOM match count improves significantly from 212/390

## Dependencies

None.

## DOM Baseline

- DTC: 789/790 matched
- opensource-guide: 212/390 matched, 1580 total diffs

## Log

### [PM] 2026-04-02 10:00
- Created from analysis of opensource-guide DOM diffs
- ~140 diff instances from combining mark stripping, ~21 from emoji stripping
- Root cause: is_alphanumeric() vs Ruby byte-level tr!
- Fix is a one-line change in slugify() filter condition
