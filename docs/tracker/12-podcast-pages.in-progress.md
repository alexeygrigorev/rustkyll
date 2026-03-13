# Issue 12: Podcast Episode Pages

## Description

Generate HTML pages for `_podcast/` using the `podcast.html` layout. This is the most complex layout (~600 lines of Liquid). Each episode gets a page at `/podcast/:title.html`.

This is a page generation wiring issue: load the podcast collection (via `load_collection("podcast", ...)`), render each episode through the `podcast.html` layout (already loaded by `LayoutEngine`), and write the resulting HTML files to the output directory. The podcast layout requires a rich site context including `site.podcast` (for episode navigation and related episodes), `site.people` (for guest bio cards and JSON-LD guest data), and `site.url`.

## Dependencies

- Issue 05 (collection loader) -- DONE
- Issue 08 (layout and includes) -- DONE
- Issue 09 (people pages -- for guest links and `site.people`) -- DONE
- Issue 10 (blog posts -- established generation patterns in `generator.rs`) -- DONE

## Scope

### In Scope

- A `generate_podcast_pages` function (or equivalent) that:
  1. Loads the `_podcast/` collection using `load_collection("podcast", ...)`
  2. Adds `site.podcast` to the site context -- an array of all podcast episode objects (needed for same-season filtering, prev/next navigation, and related episodes)
  3. Adds `site.people` to the site context (needed for guest bio cards and JSON-LD guest Person schemas)
  4. For each `CollectionItem`, renders through `LayoutEngine::render_page("podcast", ...)` passing episode front matter and the site context
  5. Writes rendered HTML to `<output_dir>/podcast/<slug>.html`

- **Episode metadata rendering**: season number, episode number, title, subtitle, description, intro text, dateadded, duration, keywords, topics

- **Platform links**: YouTube, Apple Podcasts, Spotify links -- each conditionally rendered only when present and not equal to `'TODO'`

- **Embedded YouTube player**: Via `{% include youtube.html video_id=page.ids.youtube %}` (already works through includes system)

- **Guest bio cards**: For each guest in `page.guests`, look up the person via `site.people | where: "short", guest_short | first`, then render:
  - Guest photo (`/{{ guest.picture }}`)
  - Guest name (`{{ guest.title }}`)
  - Guest bio (`guest.bio_short` or `guest.content`)
  - Social links (LinkedIn, Twitter/X, DataTalks.Club profile link)

- **Episode navigation (prev/next in season)**: The layout filters `site.podcast | where: "season", page.season | sort: "episode"`, then iterates to find the previous and next episodes relative to the current one. Each nav link shows direction arrow, episode title, and season/episode meta.

- **Tab switching UI**: Three tabs -- "Show Notes", "Timestamps", "Transcript" (transcript tab only shown if `page.transcript` exists). The JavaScript for tab switching and timestamp click-to-seek is inline in the layout.

- **Show Notes tab**: Renders `page.intro` paragraph followed by `{{ content }}` (the markdown body)

- **Timestamps tab**: Extracts headers from the transcript array -- for each `header` entry followed by a line entry with `sec`, renders a clickable timestamp link

- **Transcript rendering**: For each item in `page.transcript`:
  - If `item.header`: render as `<h3>` with slugified id
  - If `item.line`: render as `<p>` with speaker name in bold, the line text, and an optional clickable timestamp link when `item.sec` is present

- **Quotable clips**: Array of `{ name, startOffset, endOffset, url }` objects -- used in the VideoObject JSON-LD `hasPart` array as `Clip` objects

- **JSON-LD PodcastEpisode schema** (inline in the layout, NOT deferred to issue 18):
  The podcast layout contains its own JSON-LD `<script>` block with a `@graph` array containing:
  1. `PodcastEpisode` -- name, image, datePublished, dateModified, description, duration, keywords, episodeNumber, author/publisher (Organization), `about` array with host Person + guest Person objects (with sameAs links), potentialAction (WatchAction/ListenAction per platform), partOfSeason, partOfSeries
  2. `PodcastSeason` -- seasonNumber, name, numberOfEpisodes, startDate, endDate, partOfSeries
  3. `VideoObject` (if YouTube link present) -- name, description, thumbnailUrl, embedUrl, contentUrl, uploadDate, duration, publisher, associatedMedia (AudioObject), transcript text, hasPart (Clip array from quotableClips)
  4. `AudioObject` (if Anchor link present) -- name, description, contentUrl, encodingFormat, uploadDate, duration, thumbnailUrl, publisher
  5. `BreadcrumbList` -- Home > Podcast > Season N > Episode title

- **Related episodes section**: Shows up to 3 other episodes from the same season (excluding current), each with image, badge, title, and first guest name

- **Newsletter CTA section**: Static HTML form (Mailchimp) -- rendered as-is from the layout

- **Inline JavaScript**: Tab switching and timestamp-to-YouTube-seek functionality -- must be preserved verbatim in output

- **Normalizing podcast episode objects**: Like the existing `normalize_array_objects` pattern for events, podcast episode objects in `site.podcast` must have all expected keys present (at least as Nil) to avoid "Unknown index" errors during Liquid iteration. Key fields that may be missing on some episodes: `season`, `episode`, `subtitle`, `intro`, `duration`, `keywords`, `dateadded`, `date`, `guests`, `transcript`, `quotableClips`, `links`, `ids`, `image`, `description`, `topics`

### Out of Scope

- Full site build orchestration (issue 19)
- Other collection page generation (issues 11, 13, 14)
- Sitemap or RSS entries (issues 16, 17)
- General JSON-LD schema system (issue 18) -- the podcast JSON-LD is self-contained in the layout template; issue 18 covers cross-cutting schema concerns

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all new and existing tests (at least 15 new tests)
- [ ] `site.podcast` is added to the site context as an array of podcast episode objects, each with front matter fields and computed `url`/`slug`/`content` fields
- [ ] `site.people` is present in the site context (reuse the same mechanism from issue 09/10)
- [ ] A function generates podcast HTML pages from `datatalksclub.github.io/` and writes them to `<output>/podcast/`
- [ ] Running the function produces 193+ HTML files in `<output>/podcast/` (matching the collection loader count, excluding `_template.md` and other underscore-prefixed files)
- [ ] Each generated HTML file is non-empty and contains valid HTML structure (`<html`, `<body`, `</html>`)
- [ ] **Episode with full metadata** -- `podcast/building-agentic-ai-engineering-tooling-retrieval-evaluation.html` contains:
  - Season 22, Episode 1 badge text
  - The title "Building Agentic AI Systems"
  - Platform links to YouTube, Apple Podcasts, Spotify
  - A YouTube iframe embed with video ID `x2AAjqz2XmM`
  - JSON-LD `<script type="application/ld+json">` block with `"@type": "PodcastEpisode"`
  - JSON-LD `partOfSeason` with `seasonNumber: 22`
  - JSON-LD `partOfSeries` with name "DataTalks.Club Podcast"
  - Guest name "Ranjitha Kulkarni" (resolved from `site.people`)
- [ ] **Episode with transcript** -- `podcast/technical-writing-for-data-scientists.html` contains:
  - A transcript tab button
  - Transcript header `<h3>` elements (e.g., "DataTalks.Club intro", "Eugene's background")
  - Transcript lines with speaker names in bold (e.g., `<b>Alexey</b>`, `<b>Eugene</b>`)
  - Timestamp links with `data-time` attributes (e.g., `data-time="0"`, `data-time="19"`)
  - The timestamps tab with clickable timestamp entries extracted from transcript headers
- [ ] **Episode with quotable clips** -- `podcast/ab-testing-and-product-experimentation.html` contains:
  - JSON-LD `VideoObject` with a `hasPart` array of `Clip` objects
  - At least one clip with `name`, `startOffset`, `endOffset`, `url` fields
- [ ] **Episode with TODO links** -- `podcast/technical-writing-for-data-scientists.html`:
  - Does NOT render Apple Podcasts or Spotify platform link chips (both are `TODO` in the source)
  - JSON-LD does NOT include ListenAction for Apple or Spotify
- [ ] **Episode navigation** -- for a mid-season episode, the output contains:
  - A "Previous episode" link with the previous episode's title and URL
  - A "Next episode" link with the next episode's title and URL
  - Navigation links include season/episode meta text
- [ ] **Guest bio cards** -- for an episode with guests, the output contains:
  - Guest image tag with the person's picture path
  - Guest name as an `<h3>`
  - Guest bio text (from `bio_short` or `content`)
  - LinkedIn and/or Twitter social links for the guest
  - A link to the guest's DataTalks.Club profile page
- [ ] **Related episodes** -- the output contains a "Related episodes" section with up to 3 episode cards from the same season, each with image, season/episode badge, title, and guest name
- [ ] **Inline JavaScript** -- the output contains the tab switching script and timestamp click handler script
- [ ] **Newsletter CTA** -- the output contains the Mailchimp subscription form

## Test Scenarios

### Unit: Podcast collection in site context
- Build a site context that includes podcast items. Verify `site.podcast` is an array of objects.
- Verify each podcast object in `site.podcast` has `url`, `slug`, `title`, `season`, `episode` fields.
- Verify podcast objects with missing optional fields (e.g., no `transcript`, no `subtitle`) have those keys as Nil rather than missing entirely.

### Unit: Podcast episode normalization
- Create a podcast episode object missing `transcript`, `guests`, `quotableClips`, `subtitle`, `duration`. Run it through normalization. Verify all keys exist (as Nil).
- Verify normalization does not overwrite fields that are already present.

### Unit: Output path generation
- Given a podcast item with slug `building-agentic-ai-engineering-tooling-retrieval-evaluation`, verify output path is `<output>/podcast/building-agentic-ai-engineering-tooling-retrieval-evaluation.html`.
- Given a slug `ab-testing-and-product-experimentation`, verify correct output path.

### Integration: Render a single podcast episode through layout
- Load the real `podcast.html` layout and includes from `datatalksclub.github.io/`.
- Load a single episode (e.g., `building-agentic-ai-engineering-tooling-retrieval-evaluation`).
- Build a site context with `site.podcast` (all episodes) and `site.people`.
- Render the episode and verify the output contains: title, season/episode badge, YouTube iframe, platform links, JSON-LD script block.

### Integration: Guest resolution from site.people
- Render a podcast episode that has `guests: [ranjithakulkarni]`.
- Verify the output contains the guest's full name resolved from `site.people`.
- Verify the guest bio card section appears with the guest's picture and bio.

### Integration: Episode navigation (prev/next)
- Render a mid-season episode (not first or last in its season).
- Verify the output contains both "Previous episode" and "Next episode" navigation links.
- Verify the navigation links contain correct episode titles and URLs.
- Render the first episode in a season. Verify only "Next episode" appears, no "Previous episode".
- Render the last episode in a season. Verify only "Previous episode" appears.

### Integration: Transcript rendering
- Render an episode with a transcript (e.g., `technical-writing-for-data-scientists`).
- Verify the transcript tab button appears.
- Verify transcript headers render as `<h3>` elements with slugified IDs.
- Verify transcript lines render with speaker name in bold and timestamp links.
- Verify the timestamps tab extracts section headers with their first timestamp.

### Integration: Episode without transcript
- Render an episode that has no `transcript` field.
- Verify the transcript tab button does NOT appear.
- Verify the timestamps tab shows "Timestamps coming soon..." fallback text.

### Integration: Quotable clips in JSON-LD
- Render an episode with `quotableClips` (e.g., `ab-testing-and-product-experimentation`).
- Parse the JSON-LD from the output.
- Verify the `VideoObject` contains a `hasPart` array with `Clip` entries matching the source data.

### Integration: TODO link filtering
- Render an episode where some platform links are `'TODO'` (e.g., `technical-writing-for-data-scientists` with `spotify: TODO`, `apple: TODO`).
- Verify those platform chips do NOT appear in the HTML.
- Verify the JSON-LD does NOT contain ListenAction entries for those platforms.

### Integration: Related episodes
- Render an episode and verify the "Related episodes" section contains up to 3 cards.
- Verify none of the related episode cards link to the current episode.
- Verify related episodes are from the same season.

### Integration: Full generation against real data
- Load the real `datatalksclub.github.io/` site, generate all podcast pages to a temp directory.
- Verify 193+ HTML files are produced.
- Verify `building-agentic-ai-engineering-tooling-retrieval-evaluation.html` exists and contains expected content.
- Verify `technical-writing-for-data-scientists.html` exists and contains transcript content.
- Verify `ab-testing-and-product-experimentation.html` exists and contains quotable clips in JSON-LD.
- Spot-check that JSON-LD script blocks are present and parseable as JSON in at least 3 files.

### Edge cases
- Episode with no `guests` field -- no guest bio cards should appear, no crash.
- Episode with a guest `short` that does not match any person in `site.people` -- the guest name in the title falls back to the raw short string; no guest bio card renders for that guest.
- Episode with no `season` field -- season-related JSON-LD (partOfSeason, PodcastSeason) should not appear; episode navigation should gracefully handle missing season.
- Episode with no `links` or empty `links` -- no platform chips, no crash.
- Episode with `links.youtube` but no `ids.youtube` -- YouTube embed may not render, but no crash.
- Underscore-prefixed files like `_template.md` and `_s12e08.md` should be excluded from generation (already handled by collection loader).

## Implementation Notes

- Follow the same pattern as `generate_posts` in `generator.rs`: load collection, build context, iterate items, resolve layout, render, write.
- The site context must be extended to include `site.podcast` (array of all podcast episode objects). This is needed because the layout does `site.podcast | where: "season", page.season | sort: "episode"` for navigation and related episodes.
- The podcast layout uses both `where` filter (for `site.people | where: "short", guest_short`) and `where_exp` filter -- both are already implemented.
- The `sort` filter is used on the podcast array (`sort: "episode"`) -- verify this works correctly with integer episode numbers.
- The layout uses `| reverse` filter on arrays -- verify this is supported.
- The layout uses `| compact` filter to remove nil values from arrays -- verify this is supported.
- The layout uses `| map: "date"` filter to extract a field from each object in an array -- verify this is supported.
- The `jsonify` filter is used extensively in the JSON-LD block -- already implemented.
- The `relative_url` filter is used for image paths -- verify it is implemented.
- The `slugify` filter is used for transcript header IDs -- verify it is implemented.
- The `strip_html` filter is used in JSON-LD for guest descriptions -- verify it is implemented.
- Podcast episode objects need normalization (like events) to ensure all expected keys exist, preventing "Unknown index" errors during Liquid template iteration. Define a `PODCAST_FIELDS` constant similar to `EVENT_FIELDS`.
- The `| size` filter on arrays is used to get `season_episode_count` -- verify this works.
- The `forloop.last` variable is used in `{% unless forloop.last %},{% endunless %}` for JSON comma separation -- verify this is supported.

## Output Verification

After building, manually inspect:
1. `<output>/podcast/building-agentic-ai-engineering-tooling-retrieval-evaluation.html` -- verify YouTube embed, platform links, guest bio card, JSON-LD with PodcastEpisode + PodcastSeason + VideoObject + AudioObject + BreadcrumbList
2. `<output>/podcast/technical-writing-for-data-scientists.html` -- verify transcript tab, transcript headers and lines with timestamps, TODO links NOT rendered, JSON-LD without ListenAction for missing platforms
3. `<output>/podcast/ab-testing-and-product-experimentation.html` -- verify quotable clips in JSON-LD VideoObject hasPart, episode navigation links
4. Compare the structure of at least one rendered page against the original Jekyll site output to ensure layout fidelity
5. Verify JSON-LD blocks are valid JSON (parseable without errors)
