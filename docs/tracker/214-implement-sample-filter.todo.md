# Issue 214: Implement `sample` Liquid filter

## Origin

Descoped from issue 197 (fix Liquid comparison type errors). The `sample` filter is a Ruby Liquid / Jekyll extension not currently implemented in rustkyll.

## Problem

The `sample` filter (random sampling from an array) is used by muan-blog in `pages/blogroll.html`:

```liquid
{% assign links = site.data.blogroll | sort: "title" | sample: site.data.blogroll.size %}
```

This causes a parse error because `sample` is not registered as a filter.

## Requirements

- Implement the `sample` filter matching Jekyll/Ruby Liquid behavior
- `sample` with no argument returns a single random element from an array
- `sample: N` returns N random elements from the array
- Register the filter in `TemplateEngine::builder()`
- Follow existing filter implementation patterns in `src/template/filters/`

## Dependencies

None.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with new tests for `sample` filter
- [ ] `sample` with no arg returns one random element
- [ ] `sample: N` returns N random elements
- [ ] Non-array input is handled gracefully
- [ ] muan-blog blogroll page renders without filter errors
