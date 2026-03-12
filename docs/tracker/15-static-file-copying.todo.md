# Issue 15: Static File Copying

## Description

Copy static files (assets/, images/, favicon files, CNAME, robots.txt) to the output directory during build. These files are served as-is without processing.

## Dependencies

- Issue 01 (project setup)

## Scope

- Copy `assets/` directory (CSS, JS files)
- Copy `images/` directory (all subdirectories)
- Copy favicon files from root
- Copy `CNAME`, `robots.txt`
- Respect `exclude` list from `_config.yml`
- Preserve directory structure
- Unit tests verifying files are copied correctly
