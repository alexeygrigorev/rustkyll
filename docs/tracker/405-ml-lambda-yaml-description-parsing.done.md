# Issue 405: ML-lambda YAML multi-line description parsing

## Problem

The ml-deployment-lambda blog post has a multi-line `description:` in frontmatter
that YAML parses as a mapping (due to unquoted colons). Issue #348 fixed the
rendering of this mapping in meta tags, but ~30 remaining diffs are from the
description text appearing incorrectly in body content.

## Scope

Investigate and fix the remaining description-related diffs after #348's fix.
This may involve how the mapping is rendered in contexts other than bare output
tags (e.g., in truncate filters, SEO tags, etc.).

## Baseline

- DTC DOM: 788/790
- Target: reduce ml-lambda diffs further

## Log

### [SWE] 2026-03-28

**Investigation: description body diffs are NOT from YAML description rendering**

1. Built rustkyll release and generated DTC site
2. Compared Jekyll vs rustkyll HTML output for `blog/ml-deployment-lambda.html`
3. Found only 2 line-level differences:
   - Lines 317-318: Syntax highlighting class differences in bash code blocks (`class='k'` vs `class='nb'`, `class='n'` vs `class='nv'`, text wrapping differences). These are syntect vs Rouge highlighter differences.
   - Line 649: Footer GitHub Pages link href (rustkyll fills URL, Jekyll leaves empty).
4. The YAML description rendering is **identical** between Jekyll and rustkyll -- both render the malformed YAML mapping as `{"Learn containerized ML deployment on AWS Lambda"=>"build, train, and serve with Docker, ECR, and SAM, plus CI/CD via GitHub Actions. Follow this proven guide."}` in meta tags, JSON-LD, and everywhere else.
5. The 132 DOM diffs are all from syntax highlighting span-level differences within the bash code block, which the DOM comparison tool counts individually (each span attribute or text difference counts as a separate diff).

**Conclusion: This issue is NOT actionable separately.** The description-related rendering is already correct (matches Jekyll exactly). All 132 diffs are from syntax highlighting differences, which fall under the scope of syntax highlighting issues, not YAML description parsing.

No code changes made. Recommend closing as not-actionable-separately.
