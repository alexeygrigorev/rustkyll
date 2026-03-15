#!/usr/bin/env bash
# generate-large-blog.sh -- Generate a synthetic Jekyll blog with 3000 posts.
#
# Creates a site in websites/large-blog-3000/ with:
#   - 3000 blog posts across 10 categories with tags
#   - default and post layouts
#   - An index page listing all posts by category
#   - A git repo (for benchmark script compatibility)
#
# The output is deterministic: running twice produces identical content.
# Idempotent: skips generation if the site already exists with 3000 posts.
#
# Usage: scripts/generate-large-blog.sh [--dir WEBSITES_DIR]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

exec "$SCRIPT_DIR/generate-synthetic-sites.sh" --only blog "$@"
