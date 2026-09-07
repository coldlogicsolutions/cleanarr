## Why

The `cleanup` command only accounts for the Radarr movie library when comparing disk usage against the threshold, and only movies can be deleted. Users who share a disk between Radarr and Sonarr cannot use the tool to manage total media usage, and TV episodes (often the larger share of space) are never candidates for removal.

## What Changes

- Add an optional Sonarr connection (`--sonarr-url`, `--sonarr-api-key`, `SONARR_URL`/`SONARR_API_KEY`, `[sonarr]` config section, `radarrer init` support).
- When Sonarr is configured, `cleanup` counts Sonarr series `sizeOnDisk` in the library total compared against `--max-size`.
- Individual TV episode files become deletion candidates alongside movies, ranked by the same strategies (`oldest`, `oldest-added`, `never-watched`, `least-watched`, `oldest-watched`).
- Plex watch data for episodes is matched via the show's TVDB/TMDB/IMDB GUID plus season and episode number, from a Plex TV library (new `--plex-tv-section` flag, auto-detected when exactly one `show` library exists).
- Deleting an episode removes its file via Sonarr and unmonitors the episode so Sonarr does not re-download it.
- New `--media all|movies|tv` flag restricts which kinds of candidates may be deleted (default `all`). Disk usage is always counted across both.
- `--exclude-tag` also honours Sonarr series tags.
- Output tables and JSON gain a Type column / `type` field and episode-specific fields (series, season, episode).

## Capabilities

### New Capabilities
- `sonarr-client`: Thin Sonarr v3 API client (series listing, episodes with files, episode file deletion, monitor toggling, tags).

### Modified Capabilities
- `disk-cleanup`: Threshold compares combined movie + TV size; episodes are candidates; new `--media`, `--plex-tv-section` flags; episode deletion semantics; tag exclusion on series.
- `plex-client`: Fetch TV shows and episodes from a `show` section; match Sonarr episodes to Plex episodes.
- `cli-interface`: New global `--sonarr-url` / `--sonarr-api-key` flags, `[sonarr]` config section, `init` saves Sonarr settings.

## Impact

- `src/config.rs`: `SonarrConfig`, `SonarrFileConfig`, config layering, `ConfigUpdates`.
- `src/main.rs`: new global flags; pass Sonarr config to cleanup.
- New `src/sonarr.rs`: Sonarr client and resource structs.
- `src/plex.rs`: show/episode fetching, `resolve_tv_section_id`, episode matching.
- `src/commands/cleanup.rs`: unified candidate model, combined size, episode deletion.
- No new crate dependencies.
