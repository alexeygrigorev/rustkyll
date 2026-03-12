# Issue 15: Static File Copying

## Description

Copy static files and directories to the output directory during build. Static files are served as-is without any processing (no front matter parsing, no template rendering). This includes the `assets/` directory (CSS, JS), the `images/` directory (all subdirectories), favicon/PWA files at the root, `CNAME`, `robots.txt`, `podcast-timestamps/`, and other non-Jekyll files.

## Dependencies

- Issue 01 (project setup) -- `.done.md`
- Issue 02 (config parsing) -- `.done.md` (needed for the `exclude` list)

## Scope

### What counts as a static file

Jekyll copies any file or directory that is NOT:
1. A file/directory starting with `_` (collections, layouts, includes, data, etc.)
2. A file/directory starting with `.` (dotfiles, `.git`, `.github`)
3. A file/directory on the `exclude` list from `_config.yml`
4. A Markdown file (`.md`) -- these are processed as pages, not copied as static files

For the DataTalks.Club site, the static files are:

**Directories:**
- `assets/` -- CSS (`styles.css`, `syntax.css`) and JS (`accordion.js`)
- `images/` -- 1248 files across subdirectories (authors, books, courses, landing, other, partners, podcast, posts, plus root images)
- `podcast-timestamps/` -- `.txt` files with podcast timestamp data

**Root-level files (favicon/PWA):**
- `android-chrome-192x192.png`
- `android-chrome-512x512.png`
- `apple-touch-icon.png`
- `browserconfig.xml`
- `favicon-16x16.png`
- `favicon-32x32.png`
- `favicon.ico`
- `mstile-150x150.png`
- `safari-pinned-tab.svg`
- `site.webmanifest`

**Root-level files (other):**
- `CNAME`
- `robots.txt`

### What must NOT be copied (excluded)

From `_config.yml` exclude list:
- `Gemfile`, `Gemfile.lock`
- `node_modules/`
- `README.md`
- `previews/`
- `scripts/`
- `env/`
- `.github/`
- `_docx`
- `Pipfile`, `Pipfile.lock`
- `Makefile`
- `.gitignore`

Additionally, these are implicitly excluded:
- All `_*` directories (collections, layouts, includes, data, config)
- All `.*` files and directories (.git, .gitignore, .cursor)
- `_config.yml` itself
- Python tooling files (`pyproject.toml`, `uv.lock`, `package-lock.json`) -- these are not in the exclude list, but our generator should only copy files that would be included in the Jekyll build. Since Jekyll excludes `pyproject.toml`, `uv.lock`, and `package-lock.json` by default (they are not recognized site files), we should match that behavior. However, Jekyll actually DOES copy unrecognized files unless they are in the exclude list, so to match Jekyll behavior exactly, these would be copied. Follow Jekyll behavior here.

### Implementation

Create a `src/static_files.rs` module with:

1. **`is_static_file(path, config) -> bool`** -- Determine if a given path should be copied as a static file. A file is static if:
   - It is not in a `_` prefixed directory
   - It is not in a `.` prefixed directory
   - It is not on the exclude list
   - It is not a `.md` file (those are pages)
   - It is not `_config.yml`

2. **`collect_static_files(source_dir, config) -> Vec<PathBuf>`** -- Walk the source directory and return all static file paths.

3. **`copy_static_files(source_dir, output_dir, config) -> Result<usize>`** -- Copy all static files from source to output, preserving directory structure. Return the number of files copied.

Key behaviors:
- Preserve directory structure: `images/books/foo.jpg` -> `_site/images/books/foo.jpg`
- Create parent directories as needed
- Binary-safe copying (images, fonts, etc.)
- The exclude list entries may have trailing `/` for directories -- handle both `scripts/` and `scripts`
- Return the count of files copied for logging

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] A new `src/static_files.rs` module exists and is registered in `src/lib.rs`
- [ ] `is_static_file` correctly identifies static files vs. excluded/special files
- [ ] `collect_static_files` returns all static files from the source directory
- [ ] `copy_static_files` copies files to the output directory preserving directory structure
- [ ] The `exclude` list from `SiteConfig` is respected -- excluded files/dirs are NOT copied
- [ ] Files in `_` prefixed directories are NOT copied
- [ ] Files in `.` prefixed directories are NOT copied
- [ ] `.md` files are NOT copied (they are pages, not static files)
- [ ] `_config.yml` is NOT copied
- [ ] `cargo test` passes with all new tests
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` reports no changes needed

## Test Scenarios

### Unit: is_static_file

- `assets/styles.css` with empty exclude list -> true
- `images/cover.jpg` with empty exclude list -> true
- `CNAME` with empty exclude list -> true
- `robots.txt` with empty exclude list -> true
- `favicon.ico` with empty exclude list -> true
- `_layouts/default.html` -> false (underscore prefix directory)
- `_config.yml` -> false (config file)
- `.gitignore` -> false (dotfile)
- `.github/workflows/ci.yml` -> false (dot-prefixed directory)
- `README.md` with exclude=["README.md"] -> false
- `index.md` -> false (markdown file, processed as page)
- `scripts/deploy.sh` with exclude=["scripts/"] -> false
- `node_modules/foo/bar.js` with exclude=["node_modules/"] -> false
- `Gemfile` with exclude=["Gemfile"] -> false
- `browserconfig.xml` with empty exclude list -> true
- `site.webmanifest` with empty exclude list -> true
- `podcast-timestamps/s01e03.txt` with empty exclude list -> true

### Unit: collect_static_files

- Create a temp directory with a mix of static files, `_` dirs, `.` dirs, `.md` files, and excluded files. Verify only static files are returned.
- Verify subdirectories are traversed (e.g., `images/books/foo.jpg` is found)
- Verify the count matches expected number of static files

### Integration: copy_static_files

- Create a temp source directory with:
  - `assets/styles.css` (with known content)
  - `images/logo.png` (binary content)
  - `CNAME` (with known content)
  - `_layouts/default.html` (should NOT be copied)
  - `README.md` (in exclude list, should NOT be copied)
  - `index.md` (should NOT be copied)
  - `.git/config` (should NOT be copied)
- Copy to a temp output directory
- Verify `assets/styles.css` exists in output with correct content
- Verify `images/logo.png` exists in output with correct content
- Verify `CNAME` exists in output with correct content
- Verify `_layouts/` does NOT exist in output
- Verify `README.md` does NOT exist in output
- Verify `index.md` does NOT exist in output
- Verify `.git/` does NOT exist in output
- Verify returned count equals 3

### Integration: real site static files

- Point `collect_static_files` at the actual `datatalksclub.github.io/` directory with the real config
- Verify `assets/styles.css` is in the list
- Verify `images/cover.jpg` is in the list
- Verify `CNAME` is in the list
- Verify `robots.txt` is in the list
- Verify `favicon.ico` is in the list
- Verify `site.webmanifest` is in the list
- Verify no `_` prefixed paths are in the list
- Verify no `.md` files are in the list
- Verify no excluded files (Gemfile, Makefile, etc.) are in the list

## Output Verification

After implementation, building a test site should show:
- The output directory contains `assets/styles.css` with the same byte content as the source
- The output directory contains `images/` with the same subdirectory structure
- Binary files (PNG, ICO, JPG) are copied byte-for-byte (not corrupted)
- No `_layouts/`, `_includes/`, `_data/`, `_posts/` etc. appear in the output
- No excluded files appear in the output
