# Issue 91: Show build progress during site generation

## Problem

When running `rustkyll serve` or `rustkyll build`, the user sees nothing until the build is complete:

```
$ uvx rustkyll serve
Building site before serving...
Build complete: 789 pages generated.
```

For a 2-second build this is fine, but for larger sites or slower machines, the user has no idea if it's working or stuck.

## Goal

Show progress during the build so the user knows something is happening. Should include:
- Phase indicators (loading config, loading collections, rendering pages, copying static files)
- Page count progress (e.g. "Rendering pages... 150/789")
- Elapsed time
- Final summary with timing breakdown

## Example output

```
$ rustkyll build
Source:      .
Destination: _site

Loading config...
Loading collections... 6 collections, 1543 items
Loading data files... 15 files
Rendering pages... 789/789
Copying static files... 1455 files
Generating sitemap... 789 entries
Generating feed... 20 entries

Build complete!
  Pages:        789
  Static files: 1455
  Time:         1.87s
```

Or with a progress bar showing the current file:
```
Rendering [████████████████████░░░░] 650/789  blog/segmentation.html
```

## Dependencies

None

## Acceptance criteria

- Build shows phase-by-phase progress (not just final result)
- Page rendering shows count or percentage
- Final summary includes page count, static file count, and total time
- Progress output goes to stderr (so stdout can be redirected)
- Quiet mode (--quiet flag) suppresses progress, only shows errors
- Works on both `build` and `serve` commands
- No performance regression from progress output
