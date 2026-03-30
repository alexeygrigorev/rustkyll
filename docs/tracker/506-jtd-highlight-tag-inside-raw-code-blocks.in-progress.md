# Issue 506: Liquid highlight tag leaking inside raw/code blocks

## Problem

On the just-the-docs code/line-numbers page, `{% highlight ruby linenos %}` and `{% endhighlight %}` tags appear literally in the output. These tags are inside `{% raw %}` blocks within fenced code blocks in the markdown source. They should be rendered as literal text (because of `{% raw %}`), but instead they appear to be partially processed or the raw/code block interaction is wrong.

### Example

Source markdown:
````markdown
```
{% raw %}{% highlight ruby linenos %}
def foo
  puts 'foo'
end
{% endhighlight %}{% endraw %}
```
````

**Jekyll** (correct): renders code block showing `{% highlight ruby linenos %}` as syntax-highlighted code text

**Rustkyll** (broken): the `{% highlight ruby linenos %}` leaks into the HTML as raw Liquid that was partially processed

### Affected Pages

- docs/ui-components/code/line-numbers/index.html (162 diffs, many from this)

This page also has extensive diffs from the `{% highlight %}` tag with `linenos` option not being fully supported for actual code highlighting (not just the documentation example).

## Root Cause

The interaction between `{% raw %}`, fenced code blocks, and `{% highlight %}` tag processing is incorrect. The `{% raw %}` should prevent any Liquid processing of its contents, but the `{% highlight %}` tag may be getting partially evaluated.

## Dependencies

- Issue #501 (the SVG escaping fix removes 7 diffs from this page too)

## Baseline

- just-the-docs: 1/47 (or higher if #501-#505 are fixed first)
- DTC: 790/790 (must not regress)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `{% highlight %}` inside `{% raw %}` inside code blocks renders as literal text
- [ ] The `{% highlight ruby linenos %}` tag (when NOT inside raw) produces syntax-highlighted code with line numbers
- [ ] DTC DOM baseline remains at 790/790

## Test Scenarios

### Unit: Raw tag inside code blocks
- Fenced code block containing `{% raw %}{% highlight ruby %}{% endraw %}` -- verify literal output
- `{% raw %}` wrapping `{% highlight %}` -- verify no Liquid processing

### Integration: line-numbers page
- Build just-the-docs, check code/line-numbers page does not have raw `{% highlight %}` in output
- Verify the documentation example shows the literal `{% highlight %}` text

## Log

### [SWE] 2026-03-30
- Root cause: `pre_render_highlight_blocks()` in src/collection.rs naively scanned for `{% highlight %}` tags without checking if they were inside `{% raw %}...{% endraw %}` blocks or fenced code blocks (``` or ~~~). This caused highlight tags that should be literal text to get processed into `<figure>` HTML.
- TDD cycle:
  - Wrote 5 tests: `test_pre_render_highlight_blocks_inside_raw`, `test_pre_render_highlight_blocks_inside_fenced_code`, `test_pre_render_highlight_wrapping_raw_highlight`, `test_pre_render_highlight_blocks_mixed_protected_unprotected`, `test_pre_render_highlight_blocks_raw_with_unicode`
  - Ran tests: all 5 FAILED as expected (highlight inside raw/fenced was being processed)
  - Implemented fix: refactored `pre_render_highlight_blocks()` to first build a list of protected byte ranges (raw blocks and fenced code blocks) via `find_protected_ranges()`, then skip `{% highlight %}` and `{% endhighlight %}` tags that fall inside protected ranges via `is_in_protected_range()`
  - Ran tests: all 5 new tests PASS, all 11 highlight tests PASS
- Build: 3346 lib tests pass, 0 fail; all integration tests pass; clippy clean; fmt clean
- Files modified: src/collection.rs only
