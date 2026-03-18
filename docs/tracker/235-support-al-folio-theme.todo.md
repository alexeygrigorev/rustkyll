# Issue 235: Support al-folio Jekyll theme

## Problem

al-folio is one of the most popular Jekyll themes (~11k GitHub stars), heavily used in academia for personal/research websites. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/alshedivat/al-folio
- **Stars:** ~11,000
- **Use case:** Academic personal websites, research portfolios
- **Notable features:** Publications via BibTeX, project cards, blog with math (MathJax/KaTeX), Jupyter notebook integration, image galleries with lightbox, multi-language support

## Tasks

1. Clone the al-folio theme demo site into `websites/al-folio/`
2. Build with both Jekyll and rustkyll
3. Run DOM comparison and record match rate
4. Identify and fix any theme-specific rendering issues

## Acceptance Criteria

- [ ] al-folio demo site cloned into `websites/al-folio/`
- [ ] Jekyll build succeeds and produces reference output
- [ ] rustkyll build succeeds without errors
- [ ] DOM comparison run and match rate recorded
- [ ] Any theme-specific Liquid filters or layout patterns identified

## Dependencies

- None (research/benchmark task)
