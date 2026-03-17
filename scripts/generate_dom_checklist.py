#!/usr/bin/env python3
"""Generate docs/dom-differences-checklist.md from dom detail files."""

import re
from collections import defaultdict, Counter
from pathlib import Path

DETAILS_DIR = Path(__file__).parent.parent / "docs" / "comparison" / "dom-details"
OUTPUT = Path(__file__).parent.parent / "docs" / "dom-differences-checklist.md"


def parse_all():
    """Return list of (site, file, [(diff_type, path, expected, actual)]) for each DIFF file."""
    results = []
    for fname in sorted(DETAILS_DIR.glob("*.txt")):
        site = fname.stem
        current_file = None
        diffs = []
        with open(fname) as f:
            for line in f:
                line = line.rstrip('\n')
                if line.startswith(("Jekyll directory:", "Rustkyll directory:", "Common HTML files:", "Only in", "Summary:")):
                    continue
                # Strip progress messages that may appear at end of lines or standalone
                line = re.sub(r'\s*Progress: \d+/\d+ files compared\.\.\.', '', line)
                if not line.strip():
                    continue
                m = re.match(r'^DIFF (.+?) \((\d+) differences?\)', line)
                if m:
                    if current_file and diffs:
                        results.append((site, current_file, diffs))
                    current_file = m.group(1)
                    diffs = []
                    continue
                if line.startswith("  ") and current_file:
                    line = line.strip()
                    m2 = re.match(r'\.\.\. and (\d+) more differences', line)
                    if m2:
                        diffs.append(("TRUNCATED", "", "", "", int(m2.group(1))))
                        continue
                    m2 = re.match(r'^(.+?): (\w+) - expected: (.+?), actual: (.+)$', line)
                    if m2:
                        diffs.append((m2.group(2), m2.group(1), m2.group(3), m2.group(4), 1))
        if current_file and diffs:
            results.append((site, current_file, diffs))
    return results


def has_layout_issue(diffs):
    """Check if a file's diffs indicate layout-not-applied."""
    for dt, path, exp, act, _ in diffs:
        if dt == "tag_name_differs" and "child[1]" in path and "'head'" in exp:
            return True
        if dt == "missing_element" and ("body" == path or "head" == path) and "'(none)'" in act:
            return True
        if dt == "expected_element_got_text" and ("'<head>'" in exp or "'<body>'" in exp):
            return True
    return False


def classify(site, filename, dt, path, exp, act, file_has_layout_issue):
    """Classify a single diff line into a root cause category."""

    # --- Layout not applied ---
    if dt == "tag_name_differs" and "child[1]" in path and "'head'" in exp:
        return "layout_not_applied"
    if dt == "tag_name_differs" and "child[2]" in path and "'body'" in exp:
        return "layout_not_applied"
    if dt == "tag_name_differs" and "child[3]" in path and "'script'" in exp:
        return "layout_not_applied"
    if dt == "missing_element" and ("body" == path.strip() or path.strip().endswith("> body") or "head" == path.strip() or path.strip().endswith("> head")) and "'(none)'" in act:
        if "'<body>'" in exp or "'<head>'" in exp or "'<script>'" in exp:
            return "layout_not_applied"
    if dt == "expected_element_got_text" and ("'<head>'" in exp or "'<body>'" in exp):
        return "layout_not_applied"
    # Extra elements from content rendered without layout
    if file_has_layout_issue and dt == "extra_element":
        return "layout_not_applied"
    if file_has_layout_issue and dt == "tag_name_differs":
        return "layout_not_applied"
    if file_has_layout_issue and dt == "missing_element":
        return "layout_not_applied"
    if file_has_layout_issue and dt == "text_differs":
        return "layout_not_applied"
    if file_has_layout_issue and dt == "attribute_differs":
        return "layout_not_applied"
    if file_has_layout_issue and dt == "extra_text":
        return "layout_not_applied"
    if file_has_layout_issue and dt == "missing_text":
        return "layout_not_applied"
    if file_has_layout_issue and dt in ("expected_element_got_text", "expected_text_got_element"):
        return "layout_not_applied"

    # --- Liquid not rendered ---
    if "{%" in act or "{{" in act:
        return "liquid_not_rendered"

    # --- SEO meta tags (og:, twitter:, description etc.) ---
    if dt in ("attribute_differs", "missing_attribute", "extra_attribute") and "meta" in path:
        return "seo_meta_tags"

    # --- JSON-LD datePublished timezone ---
    if dt == "jsonld_value_differs" and "datePublished" in path and "+00:00" in act:
        return "jsonld_date_timezone"

    # --- JSON-LD FAQ answer text ---
    if dt == "jsonld_value_differs" and "mainEntity" in path and "acceptedAnswer" in path:
        return "jsonld_faq_text"

    # --- JSON-LD author description ---
    if dt == "jsonld_value_differs" and "author" in path and "description" in path:
        return "jsonld_author_desc"

    # --- JSON-LD missing/extra fields ---
    if dt == "jsonld_missing_field":
        return "jsonld_missing_field"
    if dt == "jsonld_extra_field":
        return "jsonld_extra_field"

    # --- JSON-LD other ---
    if dt == "jsonld_value_differs":
        return "jsonld_other"

    # --- Permalink .html extension ---
    if dt == "attribute_differs" and "href=" in exp:
        exp_m = re.search(r"href='([^']*)'", exp)
        act_m = re.search(r"href='([^']*)'", act)
        if exp_m and act_m:
            e, a = exp_m.group(1), act_m.group(1)
            if not e.endswith('.html') and a == e + '.html':
                return "permalink_html_ext"
            if not e.endswith('.html') and a.endswith('.html') and a.replace('.html', '') == e:
                return "permalink_html_ext"
            # Absolute vs relative URL
            if e.startswith("http") and a.startswith("/"):
                return "redirect_abs_vs_rel"

    # --- Redirect URL absolute vs relative ---
    if dt == "text_differs" and "location=" in exp and "location=" in act:
        return "redirect_abs_vs_rel"
    if dt == "attribute_differs" and "content=" in exp and "url=" in exp:
        if "http" in exp and "/" in act and "http" not in act:
            return "redirect_abs_vs_rel"

    # --- Syntax highlighting / code content ---
    if "code" in path:
        if dt == "attribute_differs" and "class=" in exp:
            return "syntax_highlight"
        if dt == "text_differs":
            return "syntax_highlight"
        if dt in ("missing_element", "extra_element"):
            return "syntax_highlight"

    # --- Date formatting (leading zeros) ---
    if dt == "text_differs" and "header" in path and "h1" in path:
        # muan-blog date format: 2023/07/11 vs 2023/7/11
        exp_clean = exp.strip("'\"")
        act_clean = act.strip("'\"")
        if re.search(r'\d{4}/\d{2}/\d{2}', exp_clean) and re.search(r'\d{4}/\d{1,2}/\d{1,2}', act_clean):
            return "date_format_leading_zeros"

    # --- Title tag (site.title | description) ---
    if dt == "text_differs" and "title" in path and "head" in path:
        if "|" in exp and "|" not in act:
            return "title_tag_format"

    # --- Body class attribute ---
    if dt == "attribute_differs" and "body" in path and "class=" in exp and "class=" in act:
        # Check if it's the body element itself
        path_parts = [p.strip() for p in path.split(">")]
        if path_parts[-1] == "body" or path == "body":
            return "body_class"

    # --- Inline code extra class ---
    if dt == "extra_attribute" and "code" in path and "highlighter-rouge" in act:
        return "inline_code_class"

    # --- URL encoding (non-ASCII chars, brackets) ---
    if dt == "attribute_differs" and "href=" in exp:
        if "%5D" in act or "%5B" in act or "%3E" in act or "%E2%80" in act or "%D0" in act or "%D1" in act:
            return "url_encoding"

    # --- Ampersand in heading IDs ---
    if dt == "attribute_differs" and "id=" in exp:
        exp_m = re.search(r"id='([^']*)'", exp)
        act_m = re.search(r"id='([^']*)'", act)
        if exp_m and act_m:
            if "amp" in act_m.group(1) and "amp" not in exp_m.group(1):
                return "ampersand_in_id"

    # --- Wiki markup not processed ---
    if dt == "text_differs" and ("'''" in exp or "''" in exp):
        return "wiki_markup"

    # --- Table rendering ---
    if dt in ("missing_element", "expected_element_got_text") and "'<table>'" in exp:
        return "table_rendering"

    # --- Markdown inline formatting ---
    if dt == "missing_element" and ("'<em>'" in exp or "'<strong>'" in exp or "'<a>'" in exp):
        return "markdown_inline"
    if dt == "expected_element_got_text" and "'<p>'" in exp:
        return "markdown_structure"
    if dt == "expected_text_got_element":
        return "markdown_structure"

    # --- Content text differences (catch-all for remaining text_differs) ---
    if dt == "text_differs" and "code" not in path:
        if "title" in path and "head" in path:
            return "title_tag_format"  # title diffs not caught above
        return "content_text_diff"

    # --- Content href differences (catch-all for remaining href attribute_differs) ---
    if dt == "attribute_differs" and "href=" in exp and "code" not in path:
        return "content_href_diff"

    # --- Extra/missing text nodes ---
    if dt == "extra_text":
        return "text_node_diff"
    if dt == "missing_text":
        return "text_node_diff"

    # --- expected_element_got_text (catch-all) ---
    if dt == "expected_element_got_text":
        return "markdown_structure"

    # --- Tag name differs (markdown structure) ---
    if dt == "tag_name_differs":
        return "markdown_structure"

    # --- Extra/missing elements in content ---
    if dt == "extra_element":
        return "extra_element"
    if dt == "missing_element":
        return "missing_element"

    # --- Remaining attribute diffs ---
    if dt == "attribute_differs":
        return "other_attribute"

    # --- Extra/missing attributes ---
    if dt == "extra_attribute":
        return "other_attribute"

    if dt == "TRUNCATED":
        return "TRUNCATED"

    return "uncategorized"


def main():
    all_files = parse_all()

    # Classify each diff and track per-file
    # category -> site -> set of files
    cat_sites = defaultdict(lambda: defaultdict(set))
    cat_samples = defaultdict(list)  # category -> [(site, file, sample_line)]

    for site, filename, diffs in all_files:
        fli = has_layout_issue(diffs)
        file_cats = set()
        for dt, path, exp, act, count in diffs:
            if dt == "TRUNCATED":
                # Assign truncated to dominant category of this file
                file_cat_counts = Counter()
                for dt2, p2, e2, a2, _ in diffs:
                    if dt2 != "TRUNCATED":
                        c = classify(site, filename, dt2, p2, e2, a2, fli)
                        file_cat_counts[c] += 1
                if file_cat_counts:
                    cat = file_cat_counts.most_common(1)[0][0]
                else:
                    cat = "uncategorized"
                cat_sites[cat][site].add(filename)
                file_cats.add(cat)
            else:
                cat = classify(site, filename, dt, path, exp, act, fli)
                cat_sites[cat][site].add(filename)
                file_cats.add(cat)
                if len(cat_samples[cat]) < 2:
                    sample = f"  {path}: {dt} - expected: {exp}, actual: {act}"
                    cat_samples[cat].append((site, filename, sample))

    # Count unique files per category
    cat_file_counts = {}
    for cat in cat_sites:
        cat_file_counts[cat] = sum(len(files) for files in cat_sites[cat].values())

    # Verify total
    all_files_set = set()
    for site, filename, _ in all_files:
        all_files_set.add((site, filename))
    print(f"Total diff files: {len(all_files_set)}")

    # Now build the checklist markdown
    # Order categories by page count descending

    # Define human-readable descriptions for each category
    descriptions = {
        "layout_not_applied": {
            "title": "Layout/template not applied",
            "desc": "Pages render as raw markdown/HTML content without the site's layout template wrapping them. The output has content elements (h1, p, ul, etc.) directly at root level instead of inside `<head>` and `<body>` elements within the layout.",
            "cause": "rustkyll fails to find or apply the correct layout template for these pages. This could be due to: missing `_layouts/` directory resolution for gem-based themes, `_includes` files not found, layout inheritance not working (`layout: page` -> `layout: default`), or data file references in templates failing and causing the template engine to output raw content.",
        },
        "seo_meta_tags": {
            "title": "SEO meta tag differences (og:, twitter:, description)",
            "desc": "Open Graph, Twitter Card, and description meta tags have different content, ordering, or presence between Jekyll and rustkyll output. This includes property/name attribute mismatches and content value differences.",
            "cause": "rustkyll's SEO tag generation (equivalent to jekyll-seo-tag plugin) produces meta tags in a different order or with different content. Specific issues include: meta tag ordering differences, og:type values, twitter:card values, and description content not matching Jekyll's seo-tag plugin output.",
        },
        "permalink_html_ext": {
            "title": "Permalink adds .html extension incorrectly",
            "desc": "Internal links have an extra `.html` extension. Jekyll generates links like `/pages/banners` but rustkyll generates `/pages/banners.html`.",
            "cause": "rustkyll's permalink/URL generation adds `.html` extensions to page URLs when the site's permalink style should omit them (e.g., `permalink: pretty` or custom permalink patterns without extensions).",
        },
        "body_class": {
            "title": "Body class attribute differs",
            "desc": "The `class` attribute on `<body>` elements differs. For example, Jekyll outputs `class='col-pages post-body'` but rustkyll outputs `class='col- post-body'` (missing the collection name in the class).",
            "cause": "rustkyll's template rendering does not correctly compute page-type CSS classes that depend on the page's collection membership, category, or layout type.",
        },
        "date_format_leading_zeros": {
            "title": "Date formatting missing leading zeros",
            "desc": "Dates formatted in templates lose leading zeros. Jekyll outputs `2023/07/11 15:27` but rustkyll outputs `2023/7/11 15:27`.",
            "cause": "rustkyll's Liquid date filter (`date: '%Y/%m/%d %H:%M'`) does not pad month, day, and hour values with leading zeros when the format string uses `%m`, `%d`, `%H` etc.",
        },
        "syntax_highlight": {
            "title": "Syntax highlighting differences",
            "desc": "Code blocks have different syntax highlighting: span elements have different CSS classes (e.g., `class='k'` vs `class='n'`), different text content within spans, or different span structure (missing/extra spans).",
            "cause": "rustkyll uses a different syntax highlighting engine or version than Jekyll's Rouge highlighter. The tokenization and classification of code tokens differs, producing different CSS class assignments and token boundaries.",
        },
        "jsonld_date_timezone": {
            "title": "JSON-LD datePublished timezone offset",
            "desc": "The `datePublished` field in JSON-LD structured data uses UTC (+00:00) instead of the local timezone offset. Jekyll outputs `2023-12-11T00:00:00+01:00` but rustkyll outputs `2023-12-11T00:00:00+00:00`.",
            "cause": "rustkyll does not apply the site's timezone configuration (from `_config.yml` `timezone` field) when formatting dates in JSON-LD output.",
        },
        "jsonld_faq_text": {
            "title": "JSON-LD FAQ answer text differences",
            "desc": "FAQ page structured data (`mainEntity[].acceptedAnswer.text`) has minor HTML differences in the answer text, such as trailing whitespace or slight HTML structure variations.",
            "cause": "rustkyll's markdown-to-HTML conversion for FAQ answers produces slightly different whitespace or HTML structure compared to Jekyll, particularly around trailing spaces and paragraph boundaries.",
        },
        "jsonld_author_desc": {
            "title": "JSON-LD author description trailing whitespace/markdown",
            "desc": "Author descriptions in JSON-LD have trailing newlines or unprocessed markdown links. Jekyll outputs clean text but rustkyll appends `\\n` or leaves `[link text](url)` as raw markdown.",
            "cause": "rustkyll does not strip trailing whitespace from front matter values or does not process markdown within author description fields before embedding them in JSON-LD.",
        },
        "jsonld_other": {
            "title": "JSON-LD other value differences",
            "desc": "Various JSON-LD fields differ: `description` truncation/content, `headline`, page title format. Includes cases where special characters like `$` get different handling.",
            "cause": "Multiple minor differences in how rustkyll generates JSON-LD structured data compared to Jekyll's jekyll-seo-tag plugin, including text truncation logic, special character handling, and field content generation.",
        },
        "jsonld_missing_field": {
            "title": "JSON-LD missing fields (url)",
            "desc": "JSON-LD structured data is missing the `url` field that Jekyll includes.",
            "cause": "rustkyll's JSON-LD generation does not include the `url` field from the page's permalink/URL. The jekyll-seo-tag plugin includes this field when `site.url` is configured.",
        },
        "jsonld_extra_field": {
            "title": "JSON-LD extra fields (name)",
            "desc": "JSON-LD structured data includes a `name` field that Jekyll does not include.",
            "cause": "rustkyll's JSON-LD generation adds a `name` field (from `site.title`) that the jekyll-seo-tag plugin does not include for this page type.",
        },
        "redirect_abs_vs_rel": {
            "title": "Redirect pages use relative URLs instead of absolute",
            "desc": "Pages generated by jekyll-redirect-from use relative URLs (`/community/`) instead of absolute URLs (`https://choosealicense.com/community/`). Affects `<link>`, `<meta>`, `<a>`, and `<script>` elements.",
            "cause": "rustkyll's redirect page generation does not prepend `site.url` to the redirect target URL, producing relative paths instead of the absolute URLs that Jekyll generates.",
        },
        "content_text_diff": {
            "title": "Content text/ordering differences (collection sort, markdown)",
            "desc": "Page body text content differs between Jekyll and rustkyll. This includes: collection items appearing in different order, markdown content being split into different text nodes, and text content shifts due to upstream element structure changes.",
            "cause": "Multiple causes: (1) Collections sorted differently (different default sort key or sort order), (2) Markdown rendering produces different paragraph/text splits, (3) Front matter values processed differently.",
        },
        "content_href_diff": {
            "title": "Content link href differences",
            "desc": "Links within page content have different href values. Includes URL encoding differences for non-ASCII characters (Cyrillic, special chars) and zero-width space handling.",
            "cause": "rustkyll URL-encodes non-ASCII characters in href attributes (producing `%D0%...` for Cyrillic) while Jekyll preserves them as-is. Also, zero-width spaces in URLs get encoded differently.",
        },
        "table_rendering": {
            "title": "Markdown table rendering failure",
            "desc": "Markdown tables are not rendered as `<table>` HTML elements. Instead, the table content appears as raw text or within other elements.",
            "cause": "rustkyll's markdown parser does not handle certain table formats, particularly pipe tables that appear inside list items or have non-standard formatting (wiki-style tables, tables with leading pipes).",
        },
        "wiki_markup": {
            "title": "Wiki-style markup not processed",
            "desc": "Wiki-style italic (`''text''`) and bold (`'''text'''`) markup is not converted to HTML `<em>` and `<strong>` elements. The raw wiki markup appears in the output text.",
            "cause": "The content uses MediaWiki-style markup conventions that Jekyll processes (likely via a plugin or pre-processor) but rustkyll does not handle.",
        },
        "markdown_inline": {
            "title": "Markdown inline formatting not applied",
            "desc": "Inline markdown formatting like `_emphasis_`, `**bold**`, and `[links](url)` is not converted to HTML. Missing `<em>`, `<strong>`, or `<a>` elements.",
            "cause": "Some markdown content is not being processed through the markdown parser, likely because it appears in a context where rustkyll doesn't apply markdown conversion (e.g., within certain Liquid output or front matter values).",
        },
        "markdown_structure": {
            "title": "Markdown block structure differences",
            "desc": "The HTML element structure produced by markdown rendering differs. Elements appear in different order, text content that should be in `<p>` tags appears as raw text, or block-level elements are wrapped differently.",
            "cause": "Differences between rustkyll's markdown parser output and Jekyll's kramdown parser for edge cases: image references with markdown links, nested lists, definition lists, and complex block-level structures.",
        },
        "text_node_diff": {
            "title": "Text node splitting differences",
            "desc": "Text content is split differently across child text nodes and elements. For example, text after a `<br>` tag may appear as a separate text node in one output but as part of the parent element's text in the other.",
            "cause": "Different handling of inline elements and text node boundaries in the HTML output. This can result from different markdown rendering or template output for elements like `<br>` tags within paragraphs.",
        },
        "extra_element": {
            "title": "Extra HTML elements in rustkyll output",
            "desc": "rustkyll generates extra HTML elements (like `<p>`, `<ul>`, `<div>`) that are not present in Jekyll's output. These appear within the page content area.",
            "cause": "Markdown rendering differences where rustkyll wraps content in additional block elements, or Liquid template processing produces extra wrapper elements.",
        },
        "missing_element": {
            "title": "Missing HTML elements in rustkyll output",
            "desc": "rustkyll output is missing HTML elements (like `<p>`, `<a>`, `<span>`) that Jekyll includes. Content may be present but not wrapped in the expected element.",
            "cause": "Markdown rendering differences where rustkyll doesn't generate certain wrapper elements, or content that should produce specific elements is handled differently.",
        },
        "liquid_not_rendered": {
            "title": "Liquid template tags appear in output",
            "desc": "Raw Liquid template syntax (`{% ... %}`, `{{ ... }}`) appears in the HTML output instead of being processed. The Liquid tags are not evaluated.",
            "cause": "rustkyll's Liquid template engine fails to process certain tags, likely `{% include %}` tags referencing files that can't be found, or custom Liquid tags/filters that are not implemented.",
        },
        "title_tag_format": {
            "title": "Title tag missing site description suffix",
            "desc": "The `<title>` element shows only the page title without the `| site.description` suffix that Jekyll appends. Jekyll outputs `Theme Name | Description` but rustkyll outputs just `Theme Name`.",
            "cause": "rustkyll's SEO tag generation or title template does not append `site.description` or `site.tagline` to the page title in the format expected by jekyll-seo-tag.",
        },
        "url_encoding": {
            "title": "URL encoding differences for special characters",
            "desc": "Non-ASCII characters in URLs (Cyrillic text, special symbols, square brackets) are percent-encoded differently. Jekyll preserves certain characters while rustkyll encodes them.",
            "cause": "rustkyll applies stricter URL encoding than Jekyll, encoding characters that Jekyll leaves as-is (particularly non-ASCII characters in fragment identifiers and path components).",
        },
        "ampersand_in_id": {
            "title": "Ampersand handling in heading IDs",
            "desc": "Heading IDs generated from text containing `&` differ. Jekyll produces `free--free-to-audit-courses` (stripping the ampersand) while rustkyll produces `free-amp-free-to-audit-courses` (converting `&` to `amp`).",
            "cause": "rustkyll's heading ID generation converts `&` to `amp` instead of stripping it like Jekyll/kramdown does.",
        },
        "inline_code_class": {
            "title": "Inline code gets extra CSS class",
            "desc": "Inline `<code>` elements inside links get an extra `class='highlighter-rouge language-plaintext'` attribute that Jekyll does not add.",
            "cause": "rustkyll's markdown renderer adds syntax highlighting classes to inline code elements when they appear in certain contexts (like inside links), while Jekyll/kramdown does not.",
        },
        "other_attribute": {
            "title": "Other attribute differences",
            "desc": "Miscellaneous attribute differences not covered by other categories, such as `alt` attribute whitespace differences on images.",
            "cause": "Various minor differences in how attributes are generated, including whitespace normalization in alt text and other attribute values.",
        },
    }

    # Sort by file count
    sorted_cats = sorted(cat_file_counts.items(), key=lambda x: -x[1])

    # Build markdown
    lines = []
    lines.append("# DOM Differences Checklist")
    lines.append("")
    lines.append("Comprehensive categorization of all DOM differences between Jekyll and rustkyll output")
    lines.append("across 35 benchmark sites. Generated from `docs/comparison/dom-details/` analysis.")
    lines.append("")
    lines.append(f"**Total files with differences: {len(all_files_set)}** (out of 9767 compared)")
    lines.append("")
    lines.append("Each category represents a distinct root cause. Categories are ordered by total page impact")
    lines.append("(most affected pages first). Note: a single file may have diffs in multiple categories,")
    lines.append("so per-category file counts do not sum to the total.")
    lines.append("")
    lines.append("---")
    lines.append("")

    for cat, fcount in sorted_cats:
        if cat == "TRUNCATED" or cat == "uncategorized":
            continue
        info = descriptions.get(cat, {"title": cat, "desc": "Uncategorized.", "cause": "Unknown."})

        sites_detail = []
        for site_name in sorted(cat_sites[cat].keys()):
            files = cat_sites[cat][site_name]
            sites_detail.append(f"{site_name} ({len(files)})")

        lines.append(f"- [ ] **{info['title']}** -- {fcount} pages")
        lines.append("")
        lines.append(f"  **Description:** {info['desc']}")
        lines.append("")
        lines.append(f"  **Root cause:** {info['cause']}")
        lines.append("")
        lines.append(f"  **Affected sites:** {', '.join(sites_detail)}")
        lines.append("")
        if cat_samples.get(cat):
            s = cat_samples[cat][0]
            lines.append(f"  **Sample diff** (`{s[0]}/{s[1]}`):")
            lines.append(f"  ```")
            lines.append(f"  {s[2]}")
            lines.append(f"  ```")
            lines.append("")

    # Summary
    lines.append("---")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"| Category | Pages affected |")
    lines.append(f"|----------|---------------|")
    for cat, fcount in sorted_cats:
        if cat == "TRUNCATED" or cat == "uncategorized":
            continue
        info = descriptions.get(cat, {"title": cat})
        lines.append(f"| {info['title']} | {fcount} |")
    lines.append("")

    OUTPUT.write_text("\n".join(lines) + "\n")
    print(f"Written to {OUTPUT}")
    print(f"Total categories: {sum(1 for c, _ in sorted_cats if c not in ('TRUNCATED', 'uncategorized'))}")
    print(f"Total diff files: {len(all_files_set)}")

    # Check for uncategorized
    if "uncategorized" in cat_file_counts:
        print(f"WARNING: {cat_file_counts['uncategorized']} files have uncategorized diffs")
        for s in cat_samples.get("uncategorized", []):
            print(f"  [{s[0]}] {s[1]}: {s[2]}")


if __name__ == "__main__":
    main()
