#!/usr/bin/env python3
"""Analyze all DOM difference detail files and categorize by root cause."""

import os
import re
import sys
from collections import defaultdict
from pathlib import Path

DETAILS_DIR = Path(__file__).parent.parent / "docs" / "comparison" / "dom-details"


def parse_detail_files():
    """Parse all detail files, return list of (site, file, diffs)."""
    all_diffs = []
    for fname in sorted(DETAILS_DIR.glob("*.txt")):
        site = fname.stem
        current_file = None
        diffs = []
        with open(fname) as f:
            for line in f:
                line = line.rstrip('\n')
                # Skip header lines
                if line.startswith("Jekyll directory:") or line.startswith("Rustkyll directory:") or line.startswith("Common HTML files:") or line.startswith("Only in"):
                    continue
                if not line.strip() or line.strip().startswith("Progress:"):
                    continue
                m = re.match(r'^DIFF (.+?) \((\d+) differences?\)', line)
                if m:
                    if current_file and diffs:
                        all_diffs.append((site, current_file, diffs))
                    current_file = m.group(1)
                    diffs = []
                    continue
                if line.startswith("  ") and current_file:
                    # Parse diff line
                    line = line.strip()
                    if line.startswith("... and "):
                        # "... and 22 more differences" - we count those but can't categorize
                        m2 = re.match(r'\.\.\. and (\d+) more differences', line)
                        if m2:
                            count = int(m2.group(1))
                            diffs.append(("TRUNCATED", "", "", "", count))
                        continue
                    # Format: path: diff_type - expected: 'x', actual: 'y'
                    m2 = re.match(r'^(.+?): (\w+) - expected: (.+?), actual: (.+)$', line)
                    if m2:
                        path = m2.group(1)
                        diff_type = m2.group(2)
                        expected = m2.group(3)
                        actual = m2.group(4)
                        diffs.append((diff_type, path, expected, actual, 1))
                    else:
                        # Couldn't parse - store raw
                        diffs.append(("UNPARSED", line, "", "", 1))
        if current_file and diffs:
            all_diffs.append((site, current_file, diffs))
    return all_diffs


def classify_diff(site, filename, diff_type, path, expected, actual):
    """Classify a single diff into a root cause category. Returns category string."""

    # 1. Layout not applied (template not found/rendered) - content rendered as raw markdown/HTML without layout
    # Signature: child[1]: tag_name_differs expected 'head' actual '<something-else>'
    if diff_type == "tag_name_differs" and "child[1]" in path and "'head'" in expected:
        return "LAYOUT_NOT_APPLIED"
    if diff_type == "tag_name_differs" and "child[2]" in path and "'body'" in expected:
        return "LAYOUT_NOT_APPLIED"
    if diff_type == "tag_name_differs" and "child[3]" in path and "'script'" in expected:
        return "LAYOUT_NOT_APPLIED"
    if diff_type == "missing_element" and "body" in path and "'<body>'" in expected and "'(none)'" in actual:
        return "LAYOUT_NOT_APPLIED"
    if diff_type == "missing_element" and "head" in path and "'<head>'" in expected and "'(none)'" in actual:
        return "LAYOUT_NOT_APPLIED"
    if diff_type == "missing_element" and "script" in path and "'<script>'" in expected and "'(none)'" in actual and "child[3]" in path:
        return "LAYOUT_NOT_APPLIED"

    # Content rendered without layout - shows raw content elements at root level
    if diff_type == "extra_element" and ("child[" not in path or ">" not in path):
        # Extra elements at root or shallow level when layout wasn't applied
        # Check if this file also has LAYOUT_NOT_APPLIED diffs
        return "LAYOUT_NOT_APPLIED_EXTRA"

    # 2. JSON-LD datePublished timezone differences (+01:00/+02:00 vs +00:00)
    if diff_type == "jsonld_value_differs" and "datePublished" in path:
        if "+00:00" in actual:
            return "JSONLD_DATE_TIMEZONE"

    # 3. JSON-LD value differs - long text truncation in comparison (FAQ answers etc)
    if diff_type == "jsonld_value_differs" and "mainEntity" in path and "acceptedAnswer" in path:
        return "JSONLD_FAQ_ANSWER_TEXT"

    # 4. JSON-LD value differs - author description trailing whitespace/markdown
    if diff_type == "jsonld_value_differs" and "author" in path and "description" in path:
        return "JSONLD_AUTHOR_DESCRIPTION"

    # 5. JSON-LD other value differences
    if diff_type == "jsonld_value_differs":
        return "JSONLD_VALUE_OTHER"

    # 6. JSON-LD missing/extra fields (seo-tag plugin differences)
    if diff_type == "jsonld_missing_field":
        return "JSONLD_MISSING_FIELD"
    if diff_type == "jsonld_extra_field":
        return "JSONLD_EXTRA_FIELD"

    # 7. SEO meta tag ordering/content differences (og:, twitter:, etc.)
    if diff_type in ("attribute_differs", "missing_attribute", "extra_attribute") and "meta" in path:
        if any(x in expected + actual for x in ["og:", "twitter:", "description", "content="]):
            return "SEO_META_TAG_DIFF"

    # 8. Redirect page URLs - site.url not applied (absolute vs relative URLs)
    if diff_type in ("attribute_differs", "text_differs") and ("redirect" in filename or "existing" in filename):
        if "href=" in (expected + actual) or "location=" in (expected + actual) or "url=" in (expected + actual):
            return "REDIRECT_URL_ABSOLUTE_VS_RELATIVE"
    # Also detect by content pattern
    if diff_type == "attribute_differs" and ("href=" in expected) and ("href=" in actual):
        exp_val = expected.strip("'\"")
        act_val = actual.strip("'\"")
        if "href=" in exp_val and "href=" in act_val:
            # Extract the URL values
            exp_url = exp_val.replace("href=", "").strip("'\"")
            act_url = act_val.replace("href=", "").strip("'\"")
            if exp_url.startswith("http") and act_url.startswith("/"):
                return "REDIRECT_URL_ABSOLUTE_VS_RELATIVE"
    if diff_type == "text_differs" and "location=" in expected and "location=" in actual:
        return "REDIRECT_URL_ABSOLUTE_VS_RELATIVE"
    if diff_type == "attribute_differs" and "content=" in expected and "url=" in expected and "url=" in actual:
        exp_val = expected.strip("'\"")
        if "http" in exp_val and "url=" in actual:
            act_val = actual.strip("'\"")
            if "/" in act_val and "http" not in act_val:
                return "REDIRECT_URL_ABSOLUTE_VS_RELATIVE"

    # 9. Syntax highlighting differences (code span class attributes differ)
    if diff_type == "attribute_differs" and "code" in path and ("span" in path or "pre" in path):
        if "class=" in expected and "class=" in actual:
            return "SYNTAX_HIGHLIGHT_CLASS"
    if diff_type == "text_differs" and "code" in path and "span" in path:
        return "SYNTAX_HIGHLIGHT_TEXT"
    if diff_type == "missing_element" and "code" in path and "span" in path:
        return "SYNTAX_HIGHLIGHT_STRUCTURE"
    if diff_type == "extra_element" and "code" in path and "span" in path:
        return "SYNTAX_HIGHLIGHT_STRUCTURE"
    if diff_type == "attribute_differs" and "span" in path and "code" in path:
        if "class=" in expected:
            return "SYNTAX_HIGHLIGHT_CLASS"

    # 10. Permalink extension (.html suffix) differences
    if diff_type == "attribute_differs" and "href=" in expected and "href=" in actual:
        exp_val = re.search(r"href='([^']*)'", expected)
        act_val = re.search(r"href='([^']*)'", actual)
        if exp_val and act_val:
            e = exp_val.group(1)
            a = act_val.group(1)
            if (e + ".html" == a) or (a + ".html" == e) or (e.rstrip('/') == a.rstrip('.html')):
                return "PERMALINK_HTML_EXTENSION"
            if e.rstrip('/') + '.html' == a or a.rstrip('/') + '.html' == e:
                return "PERMALINK_HTML_EXTENSION"

    # 11. Collection/page sort order differences
    if diff_type == "text_differs" and ("div > div" in path or "main" in path):
        # Check if it looks like content appearing in wrong order
        pass

    # 12. Inline formatting (emphasis/bold) markdown not processed
    if diff_type == "text_differs" and ("'''" in expected or "'''" in actual or "''" in expected or "''" in actual):
        return "WIKI_MARKUP_NOT_PROCESSED"

    # 13. Smart quotes / special char encoding
    if diff_type == "text_differs" and (chr(0x200b) in expected or chr(0x200b) in actual):
        return "MARKDOWN_INLINE_FORMATTING"

    # 14. Markdown processing differences (emphasis, links rendering)
    if diff_type == "missing_element" and ("'<em>'" in expected or "'<strong>'" in expected or "'<a>'" in expected):
        return "MARKDOWN_INLINE_FORMATTING"
    if diff_type == "expected_element_got_text" and ("'<em>'" in expected or "'<p>'" in expected or "'<head>'" in expected):
        pass  # will be caught by other rules

    # 15. Table rendering differences
    if diff_type in ("missing_element", "expected_element_got_text") and "'<table>'" in expected:
        return "TABLE_RENDERING"

    # 16. URL encoding differences ([ ] vs %5B %5D)
    if diff_type == "attribute_differs" and "href=" in expected:
        if "]'" in expected and "%5D'" in actual:
            return "URL_ENCODING_DIFF"
        if "%5D" in actual or "%5B" in actual:
            return "URL_ENCODING_DIFF"

    # 17. HTML entity / ampersand in IDs
    if diff_type == "attribute_differs" and "id=" in expected:
        if "&" in expected and "amp" in actual:
            return "AMPERSAND_IN_ID"
        if "--" in expected or "amp" in actual:
            exp_val = re.search(r"id='([^']*)'", expected)
            act_val = re.search(r"id='([^']*)'", actual)
            if exp_val and act_val:
                e = exp_val.group(1)
                a = act_val.group(1)
                if e.replace("&", "amp").replace(" ", "-") == a or e.replace("-", "amp-") != a:
                    if "amp" in a and "amp" not in e:
                        return "AMPERSAND_IN_ID"

    # 18. Liquid template not rendered (raw {% %} in output)
    if diff_type in ("expected_element_got_text", "extra_text") and ("{%" in actual or "{{" in actual):
        return "LIQUID_NOT_RENDERED"
    if diff_type == "text_differs" and ("{%" in actual or "{{" in actual):
        return "LIQUID_NOT_RENDERED"

    # 19. Code highlighting: highlighter-rouge class on inline code
    if diff_type == "extra_attribute" and "code" in path and "highlighter-rouge" in actual:
        return "INLINE_CODE_EXTRA_CLASS"

    # 20. Jekyll version string in meta tag
    if diff_type == "attribute_differs" and "Jekyll v" in expected and "Jekyll v" in actual:
        return "JEKYLL_VERSION_META"

    # 21. og:locale / article:published_time meta tag ordering
    if diff_type in ("attribute_differs", "missing_attribute", "extra_attribute") and "meta" in path:
        return "SEO_META_TAG_DIFF"

    # 22. body class attribute differences
    if diff_type == "attribute_differs" and path.endswith("body") and "class=" in expected:
        return "BODY_CLASS_DIFF"
    if diff_type == "attribute_differs" and "> body" in path and "class=" in expected:
        return "BODY_CLASS_DIFF"

    # 23. Markdown rendering: paragraph/element structure differences (text split differently)
    if diff_type == "expected_element_got_text":
        if "'<head>'" in expected:
            return "LAYOUT_NOT_APPLIED"
        if "'<body>'" in expected:
            return "LAYOUT_NOT_APPLIED"
        if "'<p>'" in expected:
            return "MARKDOWN_STRUCTURE_DIFF"
        if "'<table>'" in expected:
            return "TABLE_RENDERING"
        return "MARKDOWN_STRUCTURE_DIFF"

    if diff_type == "expected_text_got_element":
        return "MARKDOWN_STRUCTURE_DIFF"

    # 24. Extra/missing text nodes (whitespace, text split across elements)
    if diff_type == "extra_text":
        if "{%" in actual or "{{" in actual:
            return "LIQUID_NOT_RENDERED"
        return "TEXT_NODE_DIFF"
    if diff_type == "missing_text":
        return "TEXT_NODE_DIFF"

    # Catch-all for remaining types
    if diff_type == "TRUNCATED":
        return "TRUNCATED"
    if diff_type == "UNPARSED":
        return "UNPARSED"

    return f"OTHER_{diff_type}"


def main():
    all_file_diffs = parse_detail_files()

    # For each file, classify all its diffs and determine the file's primary category
    # A file's diffs may span multiple categories

    # category -> {site -> set(files)}
    category_sites = defaultdict(lambda: defaultdict(set))
    # category -> list of (site, file, diff_sample)
    category_samples = defaultdict(list)
    # category -> total diff count
    category_diff_count = defaultdict(int)

    # Track per-file categories to count unique files per category
    file_categories = defaultdict(set)  # (site, file) -> set of categories

    for site, filename, diffs in all_file_diffs:
        for diff_type, path, expected, actual, count in diffs:
            if diff_type == "TRUNCATED":
                # We need to guess the category based on the file's other diffs
                # For now, assign to the most common category of this file's other diffs
                file_cats = set()
                for dt2, p2, e2, a2, c2 in diffs:
                    if dt2 != "TRUNCATED":
                        cat = classify_diff(site, filename, dt2, p2, e2, a2)
                        file_cats.add(cat)
                # Distribute truncated diffs proportionally across non-LAYOUT categories
                # But for simplicity, assign to most likely based on what we've seen
                if file_cats:
                    # Pick the most common category from this file
                    cat_counts = defaultdict(int)
                    for dt2, p2, e2, a2, c2 in diffs:
                        if dt2 != "TRUNCATED":
                            cat = classify_diff(site, filename, dt2, p2, e2, a2)
                            cat_counts[cat] += 1
                    most_common = max(cat_counts, key=cat_counts.get)
                    category = most_common
                else:
                    category = "TRUNCATED_UNKNOWN"
                category_sites[category][site].add(filename)
                category_diff_count[category] += count
                file_categories[(site, filename)].add(category)
                continue

            category = classify_diff(site, filename, diff_type, path, expected, actual)

            # Handle LAYOUT_NOT_APPLIED_EXTRA: check if same file has LAYOUT_NOT_APPLIED
            if category == "LAYOUT_NOT_APPLIED_EXTRA":
                # Check if this file has layout-not-applied diffs
                has_layout = False
                for dt2, p2, e2, a2, c2 in diffs:
                    if dt2 != "TRUNCATED":
                        cat2 = classify_diff(site, filename, dt2, p2, e2, a2)
                        if cat2 == "LAYOUT_NOT_APPLIED":
                            has_layout = True
                            break
                if has_layout:
                    category = "LAYOUT_NOT_APPLIED"
                else:
                    # Genuine extra element not related to layout
                    category = f"OTHER_{diff_type}"

            category_sites[category][site].add(filename)
            category_diff_count[category] += count
            file_categories[(site, filename)].add(category)

            if len(category_samples[category]) < 3:
                sample = f"{path}: {diff_type} - expected: {expected}, actual: {actual}"
                category_samples[category].append((site, filename, sample))

    # Count files per category
    category_file_count = {}
    for cat in category_sites:
        total_files = sum(len(files) for files in category_sites[cat].values())
        category_file_count[cat] = total_files

    # Print summary sorted by file count
    print("=" * 80)
    print("ROOT CAUSE CATEGORIES (sorted by file count)")
    print("=" * 80)

    total_files_accounted = set()
    for cat, fcount in sorted(category_file_count.items(), key=lambda x: -x[1]):
        sites_detail = []
        for site_name in sorted(category_sites[cat].keys()):
            files = category_sites[cat][site_name]
            sites_detail.append(f"{site_name}({len(files)})")
            for f in files:
                total_files_accounted.add((site_name, f))

        print(f"\n{cat}: {fcount} files, {category_diff_count[cat]} diffs")
        print(f"  Sites: {', '.join(sites_detail)}")
        if category_samples[cat]:
            s = category_samples[cat][0]
            print(f"  Sample [{s[0]}] {s[1]}:")
            print(f"    {s[2]}")

    print(f"\n\nTotal unique files with diffs accounted: {len(total_files_accounted)}")

    # Also print all files grouped only by file for verification
    all_diff_files = set()
    for site, filename, diffs in all_file_diffs:
        all_diff_files.add((site, filename))
    print(f"Total unique files with diffs in detail files: {len(all_diff_files)}")

    unaccounted = all_diff_files - total_files_accounted
    if unaccounted:
        print(f"\nUnaccounted files ({len(unaccounted)}):")
        for s, f in sorted(unaccounted)[:10]:
            print(f"  {s}/{f}")


if __name__ == "__main__":
    main()
