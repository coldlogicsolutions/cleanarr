## 1. Dependencies & Configuration

- [x] 1.1 Add `quick-xml` with serde feature to `Cargo.toml`
- [x] 1.2 Add Plex config fields to `src/config.rs`: `[plex]` section with `url` and `token`, plus `PlexConfig` struct
- [x] 1.3 Add `--plex-url` and `--plex-token` global CLI flags to `Cli` struct in `src/main.rs` with `PLEX_URL`/`PLEX_TOKEN` env vars
- [x] 1.4 Wire Plex config through `AppConfig::load()` with same layering as Radarr (CLI > env > file), returning `Option<PlexConfig>`

## 2. Plex API Client

- [x] 2.1 Create `src/plex.rs` with `PlexClient` struct holding base URL, token, and reqwest client
- [x] 2.2 Define XML deserialization structs: `MediaContainer`, `Directory` (library sections), `Video` (movies), `Guid` (external IDs)
- [x] 2.3 Implement `PlexClient::get_libraries()` — `GET /library/sections`, returns Vec of library sections
- [x] 2.4 Implement `PlexClient::get_movies(section_id)` — `GET /library/sections/{id}/all?includeGuids=1`, returns Vec of movies with watch stats
- [x] 2.5 Implement GUID extraction: parse `tmdb://`, `imdb://` prefixes from Guid elements into typed `PlexExternalIds` struct
- [x] 2.6 Implement `PlexClient::find_movie_sections()` — filter libraries to `type="movie"` for auto-detection
- [x] 2.7 Implement helper to resolve section ID: use provided `--plex-section`, or auto-detect if single movie library, or error if multiple

## 3. Plex CLI Commands

- [x] 3.1 Create `src/commands/plex.rs` with `PlexCommand` enum: `Libraries`, `Movies`, `Stats`
- [x] 3.2 Implement `plex libraries` subcommand with table/json/quiet output
- [x] 3.3 Implement `plex movies` subcommand with `--section` flag, table showing Rating Key, Title, Year, View Count, Last Viewed
- [x] 3.4 Implement `plex stats --movie-id <id>` subcommand: fetch Radarr movie, match to Plex via TMDB/IMDB, display watch stats
- [x] 3.5 Wire `Plex` variant into `Commands` enum in `main.rs` and dispatch to `commands::plex::run()`
- [x] 3.6 Handle "Plex not configured" error gracefully for all plex subcommands

## 4. Movie Matching

- [x] 4.1 Create matching function: given Radarr movies (with tmdbId/imdbId) and Plex movies (with GUIDs), produce a HashMap<radarr_id, PlexWatchData>
- [x] 4.2 Match by TMDB ID first, fall back to IMDB ID
- [x] 4.3 Handle unmatched movies (no Plex data) — return None for watch data

## 5. Cleanup Integration

- [x] 5.1 Add `--plex-section` flag to `CleanupArgs`
- [x] 5.2 Remove `least-activity` and `oldest-activity` strategies; replace with `never-watched`, `least-watched`, `oldest-watched` in strategy validation
- [x] 5.3 Remove Radarr history fetching code (semaphore-based concurrent history requests) — no longer needed
- [x] 5.4 When Plex is configured and a Plex-aware strategy is used, fetch Plex movies and build match map
- [x] 5.5 Implement `never-watched` strategy: filter to viewCount=0, sort by added date ascending
- [x] 5.6 Implement `least-watched` strategy: sort by viewCount ascending, then lastViewedAt ascending
- [x] 5.7 Implement `oldest-watched` strategy: filter to viewCount > 0, sort by lastViewedAt ascending (watched longest ago first)
- [x] 5.8 Add View Count and Last Watched columns to cleanup table output when Plex data is available
- [x] 5.9 Add Plex watch data to JSON cleanup output when available
- [x] 5.10 Error clearly when `never-watched`, `least-watched`, or `oldest-watched` is used without Plex configured

## 6. Testing

- [x] 6.1 Test `plex libraries` against real Plex instance
- [x] 6.2 Test `plex movies` against real Plex instance, verify viewCount and lastViewedAt values
- [x] 6.3 Test `plex stats --movie-id` with a known Radarr movie that exists in Plex
- [x] 6.4 Test `cleanup` with `--strategy never-watched` dry run
- [x] 6.5 Test `cleanup` with `--strategy least-watched` dry run
- [x] 6.6 Test `cleanup` with `--strategy oldest-watched` dry run
- [x] 6.7 Verify `--strategy oldest-added` still works without Plex configured

## 7. Plex Login & Config Persistence

- [x] 7.1 Add `Serialize` derive to config structs (`ConfigFile`, `ConnectionConfig`, `OutputConfig`, `CleanupConfig`, `PlexFileConfig`) with `skip_serializing_if` for Option fields
- [x] 7.2 Add `config_path()` public function to `src/config.rs`
- [x] 7.3 Add `ConfigUpdates` struct and `save_to_config()` function: read existing config, merge updates, write back TOML
- [x] 7.4 Make `load_config_file()` public for reuse by login command
- [x] 7.5 Add `Login` variant to `PlexCommand` enum in `src/commands/plex.rs`
- [x] 7.6 Implement `login()` async function: POST to plex.tv/api/v2/pins, open browser, poll for authToken (5-min timeout), save to config
- [x] 7.7 Add `open_browser()` helper with platform-specific browser opening (macOS `open`, Linux `xdg-open`, Windows `cmd /c start`)
- [x] 7.8 Add `Init` variant to `Commands` enum in `src/main.rs`
- [x] 7.9 Handle `Init` and `Plex Login` commands before `AppConfig::load()` in `main()` so they don't require valid Radarr config
- [x] 7.10 Test `init` with --url, --api-key, --plex-url flags — verify config file is written correctly
- [x] 7.11 Test `init` merging — verify running `init` with new fields preserves existing config values
- [x] 7.12 Test `plex libraries` works using config file (no CLI flags needed)

## 8. Default "oldest" Strategy

- [x] 8.1 Add `oldest` to strategy validation in cleanup.rs
- [x] 8.2 Change default strategy from `oldest-added` to `oldest`
- [x] 8.3 Add `effective_date()` helper: returns last watched date (epoch→YYYY-MM-DD) if available, otherwise first 10 chars of added date
- [x] 8.4 Implement `oldest` sorting using `effective_date()` — movies with and without Plex data interleaved correctly
- [x] 8.5 Fetch Plex data for `oldest` strategy when Plex is configured (optional, not required like Plex-only strategies)
- [x] 8.6 Test `oldest` strategy with Plex configured — verify watched movies sort by last watched, unwatched by added date
