# Issue 55: serve command should default to current directory

## Problem

`rustkyll serve` requires `--source /path/to/site` to work. Jekyll's `serve` command defaults to the current directory — you just `cd` into your site and run `jekyll serve`.

## Goal

Match Jekyll's behavior: `rustkyll serve` (with no `--source` flag) should serve the site in the current working directory. Same for `rustkyll build`.

## Expected behavior

```bash
cd /path/to/my-site
rustkyll serve --port 4000
# Should work, building and serving the site in the current directory
```

The `--source` flag should still work as an override, just like Jekyll.

## Dependencies

None

## Acceptance criteria

- `rustkyll serve` with no `--source` uses the current working directory
- `rustkyll build` with no `--source` uses the current working directory
- `--source` flag still works as an override
- Matches Jekyll's behavior for default source directory
- All existing tests still pass
