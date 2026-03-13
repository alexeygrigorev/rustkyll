# Issue 35: Find and Test Complex Jekyll Websites

## Problem

We currently only test against 4 relatively simple Jekyll sites (minima, beautiful-jekyll, minimal-mistakes, choosealicense.com). We need complex, real-world Jekyll sites comparable to datatalks.club to stress-test rustkyll.

## Requirements

- Research and find 5-10 complex open-source Jekyll websites (large page counts, multiple collections, data files, custom plugins, heavy template logic)
- Good candidates: project documentation sites, conference sites, organization homepages, large blogs
- Clone each (shallow) into `websites/` directory
- Attempt `rustkyll build` on each
- Document results: which build, which fail, what the blockers are
- Create follow-up issues for any new feature gaps discovered
- Sites should exercise features like: pagination, categories/tags, multiple collections, data-driven pages, custom includes, Sass/SCSS, plugins

## Candidate Sources

- GitHub Pages showcase / popular Jekyll sites lists
- Large open-source project docs built with Jekyll
- Government/nonprofit sites known to use Jekyll (e.g., 18F, NHS)
- Conference/meetup sites with schedules, speakers, talks collections

## Success Criteria

- At least 5 complex sites identified and tested
- Clear report of build results per site
- Any new blockers filed as issues in the tracker
