# Issue 12: Podcast Episode Pages

## Description

Generate HTML pages for `_podcast/` using the `podcast.html` layout. This is the most complex layout (~600 lines of Liquid). Each episode gets a page at `/podcast/:title.html`.

## Dependencies

- Issue 05 (collection loader)
- Issue 08 (layout and includes)
- Issue 09 (people pages -- for guest links)

## Scope

- Render `_layouts/podcast.html` for each episode
- Episode metadata: title, season, episode number, guests
- Platform links (Apple, Spotify, YouTube, Anchor)
- Embedded YouTube player
- Guest bio cards (linked to people pages)
- Episode navigation (previous/next in season)
- Intro text rendering
- Quotable clips with timestamps
- Transcript rendering with timestamps and speakers
- JSON-LD PodcastEpisode schema (episode, season, series, video, audio, actions)
- Tab switching UI (show notes / timestamps / transcript)
- Test with 3+ actual episodes
