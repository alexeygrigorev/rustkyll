# Issue 177: Fix syntax highlighting token classes for XML/HTML code blocks

## Problem

On alexeygrigorev/mlwiki.org, XML/HTML code blocks have different syntax highlighting token CSS classes compared to Jekyll/Rouge. For example, `<span class='nt'>` (name.tag) in Jekyll becomes `<span class='p'>` or `<span class='na'>` in rustkyll.

This affects ~434 pages on mlwiki.org with code blocks containing XML/HTML/Maven POM content.

## Root cause

The syntect-to-Rouge token mapping (issue #113) does not correctly map XML-specific token types. For XML content:
- Jekyll/Rouge: `<dependencies>` gets class `nt` (name.tag)
- Rustkyll/syntect: `<` gets class `p` (punctuation), `dependencies` gets class `na` (name.attribute)

The syntect tokenizer treats XML differently from Rouge, splitting tags into punctuation + name rather than treating the whole `<tagname>` as a name.tag.

## Affected sites

| Site | Files affected | Diffs |
|------|---------------|-------|
| alexeygrigorev/mlwiki.org | ~434/639 | ~20,000+ |
| alexeygrigorev/mlbookcamp-page | ~5/15 | ~40 |
| mojombo-blog | 1/17 (tomdoc) | 8 |

## Acceptance criteria

- [ ] XML/HTML code blocks produce token classes matching Rouge output for common patterns
- [ ] `<tagname>` is tokenized as `nt` (name.tag), not split into `p` + `na`
- [ ] mlwiki.org ANTLR4_Maven.html code blocks have matching token classes (spot-check)
- [ ] Existing tests continue to pass

## Dependencies

Extends issue #113 (syntect-rouge-token-mapping) which is already done.

## Log

### [SWE] 2026-03-17

- Investigated syntect output for XML/HTML: `<tagname>` is split into `<span class="p">&lt;</span><span class="na">tagname</span><span class="p">&gt;</span>` while Rouge produces `<span class="nt">&lt;tagname&gt;</span>`
- Verified Rouge output against mlwiki.org/_site/index.php/ANTLR4_Maven.html and XML.html
- Root cause: syntect's XML grammar splits tags into punctuation + entity.name.tag tokens; the existing scope map correctly maps entity.name.tag to `na` but Rouge treats the entire `<tagname>` as `nt`
- Implemented `postprocess_xml_tag_tokens()` post-processing function that:
  - Merges `<span class="p">&lt;</span><span class="na">TAG</span><span class="p">&gt;</span>` into `<span class="nt">&lt;TAG&gt;</span>` for simple tags
  - Merges `<span class="p">&lt;/</span><span class="na">TAG</span><span class="p">&gt;</span>` into `<span class="nt">&lt;/TAG&gt;</span>` for closing tags
  - Converts `<span class="p">&lt;</span><span class="na">TAG</span> attrs...` into `<span class="nt">&lt;TAG</span> attrs...` for tags with attributes
  - Converts remaining `<span class="p">&gt;</span>` to `<span class="nt">&gt;</span>` for attribute tag closing
  - Merges attribute name + equals sign: `<span class="na">attr</span><span class="pi">=</span>` to `<span class="na">attr=</span>`
  - Normalizes string class from `s2` to `s` for XML/HTML attributes
- Added `is_xml_like_language()` helper covering xml, html, htm, xhtml, svg, xsd, xslt, rss, opml
- TDD: wrote 6 failing tests first, then implemented fix, all 6 pass
- Tests added: 6 unit tests for XML/HTML tag token merging
- Build: 1414 unit + all integration tests pass, 0 failures, clippy clean, fmt clean
- Files modified: src/syntax.rs
