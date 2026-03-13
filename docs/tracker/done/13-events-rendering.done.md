# Issue 13: Events Rendering

## Description

Render events from `_data/events.yaml` and `_data/events_extra.yaml`. Events appear on the events page (`events.md`), homepage (`index.md`), and author pages (related events). Support time-based filtering (upcoming vs past) and draft filtering.

Events are NOT a collection (no `_events/` directory). They live in `_data/events.yaml` as a YAML sequence and are accessed in templates via `site.data.events`. The data loading (issue 04) and template engine (issues 06-08) already handle this. The main work is ensuring the `event.html` include renders correctly and that time-based comparisons work in Liquid templates.

## Dependencies

- Issue 04 (data file loading) -- DONE
- Issue 08 (layout and includes) -- DONE
- Issue 12 (podcast pages) -- DONE (needed because `index.md` references `site.podcast`)

## Scope

### What already works

- `_data/events.yaml` is loaded into `site.data.events` as a Liquid array (via `data::load_data` + `build_site_context`)
- `site.time` is set to the current build time as `"YYYY-MM-DD HH:MM:SS"` string in `build_site_context`
- `_includes/event.html` is loaded as a partial by `LayoutEngine`
- `_includes/authors.html` is loaded and working (used in people/podcast/books pages)
- `where_exp` filter is implemented
- `sort` and `reverse` filters work
- `date_to_string` filter is implemented

### What needs to be implemented or fixed

1. **Time comparison in Liquid templates**: The `events.md` page uses `event.time > site.time` and `event.time <= site.time` for filtering. In the YAML data, `event.time` is a datetime like `2026-03-17 17:00:00` which serde_yaml parses as a string. `site.time` is also a string. Liquid string comparison must work correctly for these ISO-ish datetime strings. **Verify** that `where_exp` with `>` and `<=` on these string values produces the correct upcoming/past split. If not, fix the comparison logic.

2. **Draft filtering**: Events may have `draft: true`. The template uses `event.draft != true`. Since not all events have a `draft` field, `normalize_arrays` should ensure the `draft` key exists on all event objects (padded with `Nil`). Verify that `event.draft != true` correctly includes events where `draft` is nil/absent and excludes events where `draft` is `true`.

3. **`event.html` include rendering**: The include at `_includes/event.html` is:
   ```liquid
   {% assign event = include.event %}
   {% if event.time <= site.time %}
     {{ event.title }}{% if include.speakers %} by {% include authors.html authors=event.speakers %}{% endif %}
     (<a href="{{ event.youtube }}" target="_blank">watch on youtube</a>{% if event.anchor %}, <a href="{{ event.anchor }}" target="_blank">listen on anchor.fm</a>{% endif %})
   {% else %}
     <a href="{{ event.link }}" target="_blank">{{ event.title }}</a>
     on {{ event.time | date_to_string }}{% if event.end %} &ndash; {{ event.end | date_to_string }}{% endif %}
     {% if include.speakers %}by {% include authors.html authors=event.speakers %}{% endif %}
   {% endif %}
   ```
   This uses `include.event` and `include.speakers` parameters. The include system must support passing these parameters. Verify that `{% include event.html event=event speakers=true %}` correctly passes both `event` (an object) and `speakers` (a boolean) to the include context.

4. **`date_to_string` on datetime strings**: Event times are strings like `"2026-03-17 17:00:00"`. The `date_to_string` filter must handle this format (not just `YYYY-MM-DD`). Verify it produces output like `17 Mar 2026`.

5. **Speaker links**: `event.speakers` is an array of strings (people slugs like `["alexeygrigorev"]`). The `authors.html` include resolves these against `site.people` via `where: "short", a`. This already works for podcast/books. Verify it also works for events.

6. **`events_extra.yaml`**: This file exists but is not referenced in any of the current page templates (`events.md`, `index.md`). It may be used in author pages. For this issue, ensure it is loaded (it already is via data loader) but do NOT add custom logic for it unless a template references it.

### Event fields (from `_data/events.yaml`)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `time` | datetime string | yes | Event start time |
| `title` | string | yes | Event title |
| `speakers` | array of strings | yes | People slugs |
| `type` | string | yes | `podcast`, `webinar`, `workshop`, `conference` |
| `link` | string | yes | Registration/event link |
| `youtube` | string | no | YouTube recording URL (past events) |
| `anchor` | string | no | Anchor.fm recording URL (past events) |
| `end` | datetime string | no | Event end time (multi-day events) |
| `draft` | boolean | no | If true, event is hidden |

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes with all new and existing tests
- [ ] The `events.md` page renders to HTML with both "Upcoming events" and "Past events" sections
- [ ] Upcoming events section contains only events where `event.time > site.time` and `event.draft != true`
- [ ] Past events section contains only events where `event.time <= site.time` and `event.draft != true`
- [ ] Each upcoming event renders as a link: `<a href="...">title</a> on DATE by SPEAKERS`
- [ ] Each past event renders as: `title by SPEAKERS (watch on youtube, listen on anchor.fm)`
- [ ] Speaker names in events are linked to `/people/SLUG.html` (via `authors.html` include)
- [ ] The `index.md` homepage renders its "Upcoming events" section using the same event data
- [ ] `date_to_string` correctly formats datetime strings like `"2026-03-17 17:00:00"` to `"17 Mar 2026"`
- [ ] Events with `draft: true` do not appear in either section
- [ ] Events without a `draft` field are included (nil != true)
- [ ] Building the site and inspecting `events.html` output shows correct HTML structure

## Verification Commands

```bash
# Build and run all tests
export PATH="$HOME/.cargo/bin:/usr/bin:/bin:/usr/local/bin:$PATH"
cargo build
cargo clippy -- -D warnings
cargo test

# Generate events.html and inspect output (in integration test)
# The test should write events.html to a temp dir and verify contents
```

## Test Scenarios

### Unit: Time comparison in where_exp

- Create a Liquid template with `where_exp: "e", "e.time > '2025-06-01 00:00:00'"` on a list of events with times before and after. Verify correct split.
- Test that string comparison of `"2026-03-17 17:00:00" > "2026-03-13 12:00:00"` evaluates to true in Liquid.
- Test that string comparison of `"2025-01-01 00:00:00" > "2026-03-13 12:00:00"` evaluates to false.

### Unit: Draft filtering

- Create events array where some have `draft: true`, some have `draft: false`, some have no `draft` key.
- Filter with `where_exp: "e", "e.draft != true"` and verify events with `draft: true` are excluded and all others are included.
- Verify that `normalize_arrays` pads the `draft` key as `Nil` on events that lack it.

### Unit: date_to_string on datetime strings

- Apply `date_to_string` filter to `"2026-03-17 17:00:00"` and verify output is `"17 Mar 2026"`.
- Apply `date_to_string` filter to `"2025-12-25 09:00:00"` and verify output is `"25 Dec 2025"`.

### Integration: event.html include rendering

- Render `{% include event.html event=event speakers=true %}` with a past event (has youtube). Verify output contains `<a href="YOUTUBE_URL"` and the event title.
- Render same include with a future event. Verify output contains `<a href="EVENT_LINK"` and `date_to_string` formatted date.
- Render an event with `anchor` field. Verify "listen on anchor.fm" link appears.
- Render an event without `anchor` field. Verify no anchor.fm link.

### Integration: events.md full page generation

- Load the real `_data/events.yaml`, build site context with `site.time` set to a known value (or use current time).
- Render the `events.md` page content through the template engine.
- Verify the output contains "Upcoming events" heading (if any upcoming exist).
- Verify the output contains "Past events" heading.
- Verify past events have youtube links.
- Verify upcoming events have registration links.
- Verify speaker names are linked to `/people/SLUG.html`.

### Integration: index.md events section

- Render the `index.md` page and verify the "Upcoming events" section appears.
- Verify the events listed match the upcoming filter criteria.

## Implementation Guidance

The events rendering mostly relies on existing infrastructure. The key areas to verify and potentially fix:

1. **String-based time comparison**: Since both `event.time` and `site.time` are strings in `"YYYY-MM-DD HH:MM:SS"` format, Liquid's string comparison (`>`, `<=`) should work correctly because these strings sort lexicographically in chronological order. Write tests to confirm this works in the `where_exp` filter.

2. **Include parameter passing**: The `event.html` include uses `include.event` (object) and `include.speakers` (boolean). The `LayoutEngine` must pass include parameters to the partial context. Check that `render_include` in `layout.rs` handles this. If includes already work for `authors.html` with `authors=episode.guests`, the mechanism is proven -- just verify it works for `event=event speakers=true` (two parameters, one object and one boolean).

3. **No new modules needed**: This issue does not require new Rust modules. It is primarily about verifying that the existing template engine, filters, and include system handle the events templates correctly, and writing comprehensive tests that prove it.

4. **If fixes are needed**: If time comparison or draft filtering does not work, fix it in the relevant filter (`where_exp`) or context module. Do not add events-specific code to `generator.rs` -- keep the generator generic.

5. **Standalone page rendering**: This issue does NOT include generating `events.html` as a standalone page file. That is issue 14. This issue focuses on the event rendering components (include, filters, data) that issue 14 will use.

## Out of Scope

- Generating `events.html` or `index.html` as output files (that is issue 14)
- Author page event sections (can be added later)
- Events from `events_extra.yaml` unless referenced by existing templates
- Calendar integration or iCal generation
