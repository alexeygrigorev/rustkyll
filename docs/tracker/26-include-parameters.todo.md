# Issue 26: Include Parameters

## Problem

Jekyll's `{% include file.html param="value" %}` creates an `include` object accessible inside the included file as `include.param` or `include["param"]`. Rustkyll's current implementation does not fully support this, causing "Unknown index" errors.

## Requirements

- Parse parameters from `{% include file.html key="value" key2=variable %}` syntax
- Inject parsed parameters into the include's rendering context as `include.key`
- Support both quoted string values and variable references
- Fix the DTC site's `include["max_posts"]` failures
- All existing tests must continue to pass

## References

- Issue #22 compatibility research, gap #9
