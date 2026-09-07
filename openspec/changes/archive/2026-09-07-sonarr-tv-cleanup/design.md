## Context

`cleanup` currently sums Radarr `sizeOnDisk`, compares it to `--max-size`, ranks movies by strategy (optionally with Plex watch data matched by TMDB/IMDB GUID), and deletes whole movies via `DELETE /api/v3/movie/{id}?deleteFiles=true`. Plex integration is a read-only XML client (`src/plex.rs`). Config layering is CLI > env > `~/.config/radarrer/config.toml`.

Sonarr exposes a v3 API that is structurally identical to Radarr's (same auth header, same error envelope, same `diskspace`/`tag` endpoints). Its data model differs: series own seasons own episodes, and disk usage lives on episode *files*.

## Goals / Non-Goals

**Goals:**
- Count TV usage in the threshold comparison when Sonarr is configured.
- Delete individual episode files as cleanup candidates, ranked with movies under one strategy.
- Reuse Plex watch data for episodes.
- Keep behaviour byte-for-byte unchanged when Sonarr is not configured.

**Non-Goals:**
- Season- or series-level deletion strategies.
- A general Sonarr CLI (series/episode subcommands). Only what cleanup needs.
- Sonarr `plex login`-style helpers.

## Decisions

**1. Separate `SonarrClient` in `src/sonarr.rs`, reusing `RadarrClient` transport.**
`RadarrClient` is already a generic authenticated JSON client; `SonarrClient` wraps an instance of it pointed at the Sonarr URL and exposes typed methods. Alternative: rename `RadarrClient` to `ArrClient`. Rejected as churn across 37 modules.

**2. Deletion granularity is the episode file.**
Matches the request and gives the finest-grained control of freed space. Candidate = one `EpisodeFile` (has `size`, `dateAdded`). Alternative: whole series (`DELETE /series/{id}?deleteFiles=true`) — too coarse; a single popular show could exceed the space to free.

**3. Delete + unmonitor.**
`DELETE /api/v3/episodefile/{id}` then `PUT /api/v3/episode/monitor {episodeIds, monitored:false}`. Without unmonitoring, Sonarr re-grabs the episode on its next RSS sync and the cleanup is undone. Radarr deletion has no analogue because the movie record itself is removed.

**4. Unified `Candidate` model.**
```
enum CandidateKind { Movie { movie_id }, Episode { episode_id, episode_file_id, series, season, episode } }
struct Candidate { kind, title, size, added, view_count: Option<u64>, last_watched: Option<i64> }
```
Existing sort/accumulate logic operates on `Candidate` unchanged; only the deletion step dispatches on `kind`. `--media` filters candidates *after* the total is computed, so disk usage always reflects both libraries.

**5. Fetching Sonarr data lazily.**
`GET /api/v3/series` gives `statistics.sizeOnDisk` per series, enough for the threshold check with one call. Episodes are fetched only when cleanup is needed, one `GET /api/v3/episode?seriesId=X&includeEpisodeFile=true` per series with files. Alternative: `GET /api/v3/episodefile?seriesId=X` — same call count but lacks episode numbers needed for Plex matching.

**6. Plex episode matching by show GUID + S/E numbers.**
Fetch shows (`/library/sections/{id}/all?type=2&includeGuids=1`) to build `ratingKey -> {tvdb,tmdb,imdb}`. Fetch episodes (`type=4`) which carry `grandparentRatingKey`, `parentIndex` (season), `index` (episode), `viewCount`, `lastViewedAt`. Match Sonarr series by TVDB, then TMDB, then IMDB; then episode by (season, episode). Two requests total regardless of library size. Alternative: match on episode `tvdb://<episodeId>` GUIDs — fragile across Plex agents.

**7. Section resolution.** `--plex-section` continues to mean the movie library. New `--plex-tv-section` selects the `show` library, auto-detected when exactly one exists; ambiguity errors mirror the movie case. If only TV candidates exist (`--media tv`) the movie section is not resolved, and vice versa.

**8. Tag exclusion.** Sonarr has its own tag table; `--exclude-tag` names are resolved against both `/api/v3/tag` endpoints. An episode is excluded when its series carries an excluded tag.

**9. Config.** `[sonarr] url, api_key`; `SONARR_URL`, `SONARR_API_KEY`; `--sonarr-url`, `--sonarr-api-key`. Both required for `Some(SonarrConfig)`, mirroring Plex. `radarrer init` accepts and saves them.

## Risks / Trade-offs

- [Many series → many episode requests] → Requests are issued only after the threshold is exceeded and only for series with `sizeOnDisk > 0`; sequential to avoid hammering Sonarr.
- [Plex show lacks a matching GUID] → Episodes get no watch data; with `oldest` they fall back to `dateAdded`, with Plex-only strategies they are treated as unmatched exactly like movies today.
- [Unmonitor leaves a season monitored, Sonarr "missing" list grows] → Documented; acceptable and reversible from Sonarr UI.
- [Radarr and Sonarr on different disks] → Threshold is still a single library-size figure, consistent with current semantics (the tool has never compared to disk free space).
- [Output shape change] → Table gains a Type column and JSON items gain `type`; `movies` key is kept and a new `items` key added so existing consumers keep working.
