## Why

The cleanup feature's "last activity" data comes from Radarr's history, which only tracks download/import events — not actual playback. To make intelligent deletion decisions, we need real watch statistics (play count, last played date) from the Plex media server that serves these files. This lets the cleanup command prioritize deleting movies that have never been watched or haven't been watched recently, rather than relying on Radarr metadata that doesn't reflect actual usage.

## What Changes

- Add a Plex API client that authenticates via `X-Plex-Token` and fetches movie library data
- Add a `plex` command group with subcommands for listing libraries, movies, watch stats, and browser-based login
- Add `plex login` subcommand that uses Plex's PIN-based auth flow to obtain a token via browser, saving it to the config file
- Add `init` top-level command to save connection settings (Radarr URL, API key, Plex URL, Plex token) to the config file
- Add config file write-back capability (read-modify-write with field merging)
- Add Plex configuration (URL + token) alongside existing Radarr config
- Integrate Plex watch data into the cleanup command's strategy sorting, replacing Radarr history-based "activity" with actual playback data
- Add `oldest` as the new default strategy: uses last watched time from Plex when available, falls back to added date — works with or without Plex configured
- Replace `oldest-activity` strategy with `oldest-watched` (Plex-based: movies watched longest ago, excluding never-watched)
- Add new cleanup strategies: `never-watched` (prioritize unwatched movies) and `least-watched` (sort by play count ascending, then oldest last-played)

## Capabilities

### New Capabilities
- `plex-client`: Plex API client supporting authentication, library listing, movie metadata retrieval including watch statistics (viewCount, lastViewedAt), and PIN-based browser auth flow via plex.tv
- `plex-cli`: CLI subcommands for interacting with Plex (library list, movie list with watch stats, browser-based login)

### Modified Capabilities
- `disk-cleanup`: New default `oldest` strategy (hybrid: last watched or added date). Replace `oldest-activity` with Plex-based `oldest-watched`. Add new strategies: `never-watched`, `least-watched`. Remove `least-activity` (superseded by Plex watch data).

## Impact

- New dependency: None (reuse existing reqwest client, Plex API returns XML so need quick-xml or parse as text)
- Config file gains `[plex]` section with `url` and `token` fields
- New CLI flags: `--plex-url`, `--plex-token` (global, with env vars `PLEX_URL`, `PLEX_TOKEN`)
- New source files: `src/plex.rs` (client), `src/commands/plex.rs` (CLI commands)
- Modified: `src/commands/cleanup.rs` (Plex-aware strategies), `src/config.rs` (Plex config + config write-back), `src/main.rs` (new subcommands + global flags)
- New top-level command: `init` for persisting connection settings to config file
- Plex movie matching to Radarr movies via TMDB/IMDB GUIDs embedded in Plex's `guid` field
