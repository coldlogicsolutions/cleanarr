## 1. Configuration

- [x] 1.1 Add `SonarrFileConfig` (`[sonarr] url, api_key`) and `SonarrConfig` to `src/config.rs`; layer CLI > env > file in `AppConfig::load`
- [x] 1.2 Extend `ConfigUpdates`/`save_to_config` and `radarrer init` with `--sonarr-url`/`--sonarr-api-key`
- [x] 1.3 Add global `--sonarr-url` (`SONARR_URL`) and `--sonarr-api-key` (`SONARR_API_KEY`) flags in `src/main.rs`

## 2. Sonarr client

- [x] 2.1 Create `src/sonarr.rs` with `SonarrClient` wrapping `RadarrClient` transport and resource structs (`SeriesResource`, `EpisodeResource`, `EpisodeFileResource`, `TagResource`)
- [x] 2.2 Implement `get_series`, `get_episodes(series_id)`, `get_tags`, `delete_episode_file(id)`, `unmonitor_episodes(ids)`

## 3. Plex TV support

- [x] 3.1 Add `Show`/`Episode` XML structs and `get_shows(section)`, `get_episodes(section)` to `src/plex.rs`
- [x] 3.2 Add `find_show_sections` and `resolve_tv_section_id`
- [x] 3.3 Implement `match_episodes` (series TVDB→TMDB→IMDB, then season/episode)

## 4. Cleanup command

- [x] 4.1 Add `--media` and `--plex-tv-section` args; validate `--media tv` requires Sonarr
- [x] 4.2 Introduce `Candidate`/`CandidateKind`; compute combined library size and print movie/TV/library sizes
- [x] 4.3 Build episode candidates from Sonarr (series with size > 0, episodes with files, series tag exclusion)
- [x] 4.4 Attach Plex watch data to movies and episodes; apply strategies over the unified list
- [x] 4.5 Update table/JSON/quiet output with Type column, `type`/episode fields, `items` key, `tvSize`/`librarySize`
- [x] 4.6 Implement deletion dispatch: Radarr movie delete; Sonarr episode file delete + unmonitor; summary counts

## 5. Verification

- [x] 5.1 `cargo build` and `cargo clippy` clean
- [x] 5.2 Unit tests for `match_episodes`, `--media` parsing, and candidate sorting
- [x] 5.3 Update `openspec/specs` via archive once verified
