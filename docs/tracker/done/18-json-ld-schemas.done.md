# Issue 18: JSON-LD Schema Markup

## Description

Generate JSON-LD structured data in `<script type="application/ld+json">` tags for SEO. The site uses multiple Schema.org types across different page types.

## Dependencies

- Issue 09 (people pages -- Person schema)
- Issue 10 (blog posts -- Article schema)
- Issue 12 (podcast pages -- PodcastEpisode schema)

## Scope

- Article schema: headline, image, datePublished, dateModified, author, publisher
- Person schema: name, image, url, sameAs (social links)
- PodcastEpisode schema: name, description, duration, episodeNumber, season, guests
- PodcastSeries and PodcastSeason schemas
- VideoObject and AudioObject schemas
- BreadcrumbList schema
- FAQPage schema (for accordion pages)
- Organization schema (publisher)
- WatchAction/ListenAction for podcast platforms
- Test JSON-LD output is valid JSON and contains expected fields
