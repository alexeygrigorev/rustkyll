# Issue 89: Fix missing content sections on DTC site

## Problem

Some sections/widgets/sidebars on the DTC site may not render with rustkyll. This could include:
- Navigation menus
- Footer content
- Sidebar widgets
- Social media links
- Newsletter signup forms
- Sponsor logos
- FAQ accordions
- Related content sections

## Goal

Every content section visible on the Jekyll-built DTC site must also appear on the rustkyll-built site with the same content.

## Approach

1. Compare each page section-by-section between Jekyll and rustkyll
2. Identify missing sections
3. Trace to the Liquid template/include responsible
4. Fix the rendering issue

## Dependencies

- Issue 87 (visual parity audit) will identify the specific missing sections

## Acceptance criteria

- All content sections that appear in Jekyll also appear in rustkyll
- Navigation (header + footer) matches
- Sidebar content matches
- No missing widgets or interactive elements
- Sponsor logos and social links present
