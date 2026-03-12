# Issue 09: People Collection Pages

## Description

Generate HTML pages for the `_people/` collection using the `author.html` layout. Each person gets a page at `/people/:title.html` with their profile, social links, related articles, events, and books.

## Dependencies

- Issue 05 (collection loader)
- Issue 08 (layout and includes)

## Scope

- Render `_layouts/author.html` for each person
- Profile picture, name, bio content
- Social links (twitter, linkedin, github, web)
- Related articles (posts where `authors` contains `person.short`)
- Related events (from `site.data.events` where `speakers` contains `person.short`)
- Related books (from `site.books` where `authors` contains `person.short`)
- JSON-LD Person schema
- Test with 3+ actual people from the site
