# Issue 13: Events Rendering

## Description

Render events from `_data/events.yaml` and `_data/events_extra.yaml`. Events appear on the events page, homepage (upcoming), and author pages (related events). Support time-based filtering (upcoming vs past) and draft filtering.

## Dependencies

- Issue 04 (data file loading)
- Issue 08 (layout and includes)

## Scope

- `{% include event.html %}` component
- Upcoming vs past event rendering (based on `event.time` vs build time)
- Draft filtering (`event.draft != true`)
- Event fields: time, title, speakers, type, link, youtube, anchor, end
- Speaker links to people pages
- YouTube/Anchor links for past events
- Registration links for future events
- Events page with upcoming and past sections
- Test with actual events data
