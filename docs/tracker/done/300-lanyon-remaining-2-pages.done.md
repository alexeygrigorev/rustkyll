# Issue 300: lanyon remaining 2 pages (4/6 -> 6/6)

## Problem

lanyon matches 4/6. The 2 failing pages are `index.html` (20 diffs) and
`2020/04/02/example-content/index.html` (4 diffs). Root causes identified
below.

## Diff Analysis

### Page 1: `index.html` -- 20 diffs

The index page renders post excerpts. The "Example content" post uses
`{% highlight js %}...{% endhighlight %}` Liquid tags for a JavaScript code
block. In Jekyll, this renders as a `<figure class="highlight"><pre><code>...
</code></pre></figure>`. In rustkyll, the `{% highlight %}` tags are NOT
processed in the excerpt context -- they appear as literal text
`{% highlight js %}` inside `<p>` tags.

This causes a cascade: the unprocessed code block creates 3 extra `<p>`
elements, shifting every subsequent element (h3, ul, ol, dl, etc.) down by 3
positions, resulting in 20 tag_name_differs and text_differs diffs.

There is also a canonical link diff: `href='http://lanyon.getpoole.com/'` vs
`href='http://lanyon.getpoole.com/index.html'`. Jekyll strips `index.html`
from the canonical URL for the homepage; rustkyll keeps it.

**Root cause:** The `{% highlight %}` Liquid tag is not being evaluated when
rendering post excerpts. The excerpt content goes through markdown rendering
but the Liquid `{% highlight %}` tag is either not registered or not executed
in the excerpt rendering pipeline.

**Secondary:** The canonical URL for the homepage should be `/` not
`/index.html`.

### Page 2: `2020/04/02/example-content/index.html` -- 4 diffs

All 4 diffs are Rouge syntax highlighting token class mismatches:

- `class='k'` (keyword) vs `class='o'` (operator) -- likely Java/JS `new` keyword
- `class='nc'` (name.class) vs `class='nb'` (name.builtin) -- class name
- `class='mi'` (number.integer) vs `class='m'` (number) -- 2 occurrences

These are syntect-to-Rouge token mapping issues in `src/syntax.rs`, the same
category as issue 293.

## Scope

### In scope

1. **`{% highlight %}` tag processing in excerpts** -- ensure the highlight
   Liquid tag is evaluated when rendering post excerpts on index/listing pages.
   This is the primary fix (resolves ~18 of 20 diffs on index.html).

2. **Canonical URL for homepage** -- strip trailing `index.html` from the
   canonical link URL when the page is the site index (resolves 1 diff).

3. **Rouge token class mapping for JS/Java** -- fix the 4 token class
   mismatches on the example-content page:
   - `new` keyword in JS/Java: should be `k` (keyword), not `o` (operator)
   - Class names after `new`: should be `nc` (name.class), not `nb`
     (name.builtin)
   - Integer literals: should be `mi` (number.integer), not `m` (number)

### Out of scope

- General Rouge token mapping improvements beyond the 4 specific diffs here
  (tracked by issue 293)

## Dependencies

- None. All fixes are independent.

## Key Files to Modify

- `src/template/layout.rs` or `src/generator.rs` -- excerpt rendering
  pipeline, ensure `{% highlight %}` Liquid tag is processed
- `src/template/tags/highlight.rs` or equivalent -- verify highlight tag is
  registered in the Liquid engine used for excerpts
- `src/generator.rs` or `src/template/seo_tag.rs` -- canonical URL generation,
  strip `index.html` for homepage
- `src/syntax.rs` -- token class mappings for JS/Java keywords, class names,
  and integer literals

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `{% highlight js %}...{% endhighlight %}` in a post excerpt renders as
      `<figure class="highlight"><pre><code>...</code></pre></figure>`, not as
      literal text
- [ ] lanyon `index.html` post excerpts match Jekyll output: the Example
      content excerpt contains a syntax-highlighted code block followed by
      correctly ordered h3/p/ul/ol/dl elements
- [ ] lanyon `index.html` canonical link URL is `http://lanyon.getpoole.com/`
      (no trailing `index.html`)
- [ ] lanyon `2020/04/02/example-content/index.html` syntax highlighting:
      - `new` keyword has `class="k"` (not `class="o"`)
      - Class name `Function` has `class="nc"` (not `class="nb"`)
      - Integer literals `2` and `6` have `class="mi"` (not `class="m"`)
- [ ] lanyon DOM comparison reaches 6/6 (100%)
- [ ] No regressions on DTC, muan-blog, choosealicense, mlwiki, or any of the
      13+ sites currently at 100%
- [ ] Tests include non-ASCII content (e.g., a highlight block with Unicode
      variable names)

## Test Scenarios

### Unit: Highlight tag in excerpts

- Create a post with `{% highlight js %}var x = 1;{% endhighlight %}` in the
  content, render the excerpt, verify the output contains
  `<figure class="highlight">` (not literal `{% highlight js %}`)
- Create a post where the highlight block is beyond the excerpt cutoff, verify
  the excerpt does not contain a broken/partial highlight tag
- Create a post with `{% highlight python %}x = "hello"{% endhighlight %}`
  (Unicode string), verify correct rendering

### Unit: Canonical URL homepage

- Configure a site with `url: http://example.com`, render the homepage,
  verify the canonical link `href` is `http://example.com/` (not
  `http://example.com/index.html`)
- Verify a non-homepage page like `/about/index.html` keeps its canonical URL
  as `/about/` (not affected)

### Unit: Rouge token classes for JS

- Highlight `var x = new Function("a", "b", "return a + b");` as JavaScript,
  verify:
  - `new` gets `class="k"` (keyword)
  - `Function` gets `class="nc"` (name.class)
  - Numeric literals get `class="mi"` (number.integer)

### Integration: lanyon site build

- Build lanyon with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify 6/6 pages match (100%)
- Spot-check `index.html` body: the Example content post excerpt must contain
  a `<figure class="highlight">` element
- Spot-check `2020/04/02/example-content/index.html`: all `<span>` elements
  in the highlighted code block must have correct Rouge classes

### Regression: Other sites

- Run `cargo test` full suite
- Run DOM comparison on DTC, muan-blog to verify no regression
- Verify all 13+ sites at 100% remain at 100%

## Output Verification

```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/lanyon/ \
  --destination /tmp/lanyon_test

python3 scripts/dom_compare.py \
  --jekyll-dir websites/lanyon/_site_jekyll_cached \
  --rustkyll-dir /tmp/lanyon_test
```

Spot-check:
- `grep 'highlight' /tmp/lanyon_test/index.html` -- must show `<figure class="highlight">`
- `grep 'class="k"' /tmp/lanyon_test/2020/04/02/example-content/index.html` -- must show `new` keyword
- Summary line must show: `6 files matched, 0 files with differences`

## Log

### [SWE] 2026-03-21
- **Fix 1: `{% highlight %}` in excerpts**
  - Wrote 3 tests (test_excerpt_highlight_tag_rendered_in_excerpt_html, test_excerpt_highlight_beyond_cutoff_not_partial, test_excerpt_highlight_unicode_content)
  - Root cause: excerpt_html in collection.rs only ran markdown_to_html, skipping Liquid engine
  - Fix: when excerpt contains `{%` or `{{`, process through TemplateEngine first, then markdown
  - File modified: src/collection.rs

- **Fix 2: Canonical URL strips index.html**
  - Wrote 3 tests (test_canonical_url_strips_index_html_for_homepage, test_canonical_url_strips_index_html_for_subdir, test_canonical_url_preserves_non_index_html)
  - Ran tests: FAIL as expected (canonical URL had /index.html)
  - Fix: strip trailing `index.html` from canonical path in seo_tag.rs
  - Ran tests: PASS
  - File modified: src/template/seo_tag.rs

- **Fix 3: Rouge token class mismatches for JS**
  - Wrote 4 tests (test_js_new_keyword_is_k, test_js_class_name_after_new_is_nc, test_js_integer_literal_is_mi, test_js_new_keyword_unicode_context)
  - Ran tests: FAIL as expected (new=o, Function=nb, 42=m)
  - Fix: added JS-specific scope overrides for support.class->nc, constant.numeric->mi; post-processing for new keyword o->k
  - Ran tests: PASS
  - File modified: src/syntax.rs

- Build: 2767 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: src/collection.rs, src/template/seo_tag.rs, src/syntax.rs
