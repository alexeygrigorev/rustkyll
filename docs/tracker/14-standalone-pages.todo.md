# Issue 14: Standalone Pages

## Description

Generate HTML for standalone `.md` pages in the site root: index.md (homepage), articles.md, books.md, people.md, podcast.md, events.md, courses.md, slack.md, support.md, tools.md.

## Dependencies

- Issue 05 (collection loader -- for listing items)
- Issue 08 (layout and includes)
- Issue 13 (events rendering -- for homepage and events page)

## Scope

- Homepage (index.md): upcoming events, latest podcasts, book of the week, latest articles, sponsors
- Articles page: list all posts
- Books page: upcoming and archive books
- People page: list all people
- Podcast page: episodes grouped by season
- Events page: upcoming and past events
- Courses, slack, support, tools pages
- Subscribe includes (Mailchimp form)
- Test that each page generates valid HTML with expected content
