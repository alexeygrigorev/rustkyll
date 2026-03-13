# Issue 25: Extended Permalink Variables

## Problem

Rustkyll only supports `:title` and `:collection` in permalink patterns. Jekyll supports `:year`, `:month`, `:day`, `:categories`, `:slug`, `:short_year`, `:i_month`, `:i_day`, and named styles (`pretty`, `date`, `ordinal`, `none`).

## Requirements

- Support `:year`, `:month`, `:day` extracted from post filename or front matter `date`
- Support `:categories` (joined with `/`)
- Support `:slug` (same as `:title` but parameterized)
- Support named permalink styles: `pretty` = `/:categories/:year/:month/:day/:title/`, `date` = `/:categories/:year/:month/:day/:title.html`, `none` = `/:categories/:title.html`
- Support per-collection permalink patterns from config
- All existing tests must continue to pass

## References

- Issue #22 compatibility research, gap #5
