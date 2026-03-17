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
