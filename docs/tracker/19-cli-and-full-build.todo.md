# Issue 19: CLI and Full Site Build

## Description

Wire everything together into the CLI binary. The command reads a source directory, builds the full site, and writes output to a destination directory. Support `--source` and `--output` flags.

## Dependencies

- Issue 02 (config), 04 (data), 05 (collections), 08 (templates), 14 (pages), 15 (static files), 16 (sitemap), 17 (RSS)

## Scope

- CLI with clap: `rustkyll build --source ./site --output ./_site`
- Build pipeline: load config → load data → load collections → load templates → render all pages → generate sitemap → generate feed → copy static files
- Progress reporting (number of pages generated)
- Error reporting with file/line context
- Integration test: build the actual DataTalks.Club site end-to-end
