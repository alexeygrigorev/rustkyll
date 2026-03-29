# Issue 517: Chirpy text-and-typography page has ~100 structural diffs

## Problem

The `posts/text-and-typography/index.html` page has 120 total diffs (after acceptable
filtering). Most stem from three root causes that compound into cascading structural
differences:

### 1. Block IAL classes not applied to headings and blockquotes (~20 diffs)

The post uses kramdown IALs extensively:
```markdown
# H1 -- heading
{: .mt-4 .mb-0 }

> prompt-tip example
{: .prompt-tip }
```

These classes are not applied to the target elements. Covered by issue #505.

### 2. Chirpy refactor-content.html image handling with lqip (~15 diffs)

The chirpy include `refactor-content.html` performs complex post-processing on images:
- Extracts `lqip` attribute from `<img>` tags
- Wraps images in `<a class="popup img-link">` wrappers
- Converts `src=` to `data-src=` for lazy loading with lqip
- Adds blur/shimmer CSS classes

This relies on Liquid string manipulation (`split`, `replace`, `append`) operating on
the rendered HTML content. If the input HTML structure differs even slightly (e.g.,
attribute ordering on `<img>` tags), the entire chain breaks.

### 3. Code block structure and header labels (~30 diffs)

The same include also post-processes code blocks:
- Removes inner `<pre>` wrapper: `<div class="highlight"><pre class="highlight"><code`
  becomes `<div class="highlight"><code`
- Adds code-header divs with language labels and copy buttons
- Extracts language from class names

This depends on rustkyll producing the exact same highlight HTML structure as Jekyll/Rouge.

### 4. Heading anchor generation (~10 diffs)

The include generates anchor links for h2-h5 headings, wrapping content in
`<span class="me-2">` and appending `<a class="anchor">` elements. This depends on
heading IDs being identical to Jekyll's output.

### 5. Mermaid diagrams and math equations (~10 diffs)

These are advanced features that may produce different HTML depending on the markdown
parser's handling of fenced code blocks with `mermaid` language and LaTeX delimiters.

## Scope

This is an investigation/tracking issue. The page touches nearly every advanced feature
of the Chirpy theme. Many of the underlying issues are already tracked:

- Block IAL: #505 (groomed)
- Rouge code structure: #471 (in-progress)
- Kramdown emphasis: #390 (in-progress)

After those issues land, re-run DOM comparison and reassess remaining diffs.

## Dependencies

- Issue #505 (block IAL class application) -- groomed
- Issue #471 (syntax highlighting token mismatches) -- in-progress
- Issue #514 (SEO tag hash image frontmatter) -- fixes og:image diffs on this page

## Baseline

- DTC: 790/790 (must not regress)
- Chirpy: 12/17 (this page is one of the 5 that differ)

## Acceptance Criteria

- [ ] Re-run DOM comparison after #505 and #514 land
- [ ] Document remaining diff count for this page
- [ ] For each remaining diff category, either fix it or create a follow-up issue
- [ ] DTC DOM baseline remains at 790/790
- [ ] Chirpy DOM match count improves (target: reduce from 120 to under 50 diffs on this page)

## Test Scenarios

### Investigation: DOM diff analysis
- Build chirpy after #505 lands, count remaining diffs on text-and-typography
- Build chirpy after #514 lands, count remaining meta tag diffs
- Categorize remaining diffs into fixable vs theme-specific vs Rouge-dependent
