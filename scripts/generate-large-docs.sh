#!/usr/bin/env bash
# generate-large-docs.sh -- Generate a synthetic Jekyll documentation site with 800 pages.
#
# Creates a site in websites/large-docs-site/ with:
#   - 800 documentation pages across 10 sections (80 pages each)
#   - default and doc layouts
#   - An index page listing all documentation pages
#   - A git repo (for benchmark script compatibility)
#
# The output is deterministic: running twice produces identical content.
# Idempotent: skips generation if the site already exists with 800 pages.
#
# Usage: scripts/generate-large-docs.sh [--dir WEBSITES_DIR]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

exec "$SCRIPT_DIR/generate-synthetic-sites.sh" --only docs "$@"
