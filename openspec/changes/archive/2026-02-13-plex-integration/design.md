## Context

Radarrer's cleanup command currently uses Radarr's history API to determine movie "activity" — but this only tracks download/import events, not actual playback. The Plex media server that serves these files has real watch data (play count, last played timestamp) accessible via its HTTP API at `GET /library/sections/{id}/all`. Movies are matched between Radarr and Plex using TMDB/IMDB GUIDs embedded in Plex's `guid` field.

The existing codebase has a thin reqwest-based `RadarrClient` in `src/client.rs` and 37 command modules following a consistent pattern with clap derive enums and three output formats (table, json, quiet).

## Goals / Non-Goals

**Goals:**
- Add a Plex API client that fetches movie watch statistics (viewCount, lastViewedAt)
- Provide CLI subcommands for inspecting Plex libraries and movie watch data
- Integrate Plex watch data into cleanup strategies for better deletion decisions
- Match Plex movies to Radarr movies via TMDB/IMDB GUIDs

**Non-Goals:**
- Full Plex API coverage (only library listing + movie metadata needed)
- Plex playback control, playlist management, or user management
- Tautulli integration (direct Plex API is sufficient for our needs)
- Modifying Plex state (read-only access)

## Decisions

### D1: XML parsing approach — quick-xml with serde

Plex API returns XML, not JSON. Use `quick-xml` with serde derive for deserialization, mirroring how we use `serde_json` for Radarr. This keeps the parsing pattern consistent and avoids manual XML traversal.

Alternative considered: Regex/string parsing of XML responses. Rejected — fragile and error-prone for nested structures.

### D2: Separate PlexClient struct in `src/plex.rs`

Create a standalone `PlexClient` (not merged into `RadarrClient`) since the APIs are fundamentally different — different auth mechanism (`X-Plex-Token` query param vs `X-Api-Key` header), different response format (XML vs JSON), different base URL. The client handles XML deserialization internally and exposes typed Rust structs.

Alternative considered: Generic API client trait. Rejected — over-engineering for two concrete clients with different auth and serialization.

### D3: GUID-based movie matching

Plex stores external IDs in a `Guid` element list on each movie (e.g., `plex://movie/...`, `imdb://tt1234567`, `tmdb://12345`). Match to Radarr movies by extracting TMDB ID from Plex GUIDs and comparing to Radarr's `tmdbId` field. Fall back to IMDB ID matching if TMDB is unavailable.

This matching happens at cleanup time, not stored persistently.

### D4: Plex config is optional — cleanup gracefully degrades

Plex URL and token are optional. When not configured:
- `plex` subcommands fail with a clear error ("Plex not configured")
- Cleanup strategies that don't need Plex data (`oldest-added`) work unchanged
- New Plex-specific strategies (`never-watched`, `least-watched`, `oldest-watched`) require Plex config and fail if missing

### D5: New cleanup strategies alongside existing ones

Replace `oldest-activity` and `least-activity` with Plex-based strategies, and add a new hybrid default:
- `oldest` (NEW DEFAULT): Uses last watched time from Plex if available, otherwise falls back to date added. Works with or without Plex — without Plex it degrades to sorting by added date. This is the best general-purpose strategy.
- `never-watched`: Movies with viewCount=0, sorted by added date (oldest first)
- `least-watched`: Sort by viewCount ascending, then lastViewedAt ascending (ties broken by least recently watched)
- `oldest-watched`: Movies that were watched longest ago (sorted by lastViewedAt ascending), excluding never-watched movies (viewCount=0). This replaces the old `oldest-activity` strategy with real playback data.

`oldest-added` remains unchanged (always uses added date, ignores Plex). The Radarr history-based strategies (`least-activity`, `oldest-activity`) are removed — they were unreliable proxies for actual usage and are fully superseded by the Plex-based strategies.

### D6: Auto-detect movie library section

When the user doesn't specify a `--plex-section` ID, auto-detect by fetching `GET /library/sections` and finding sections with `type="movie"`. If exactly one movie library exists, use it automatically. If multiple exist, require the user to specify which one.

### D7: Plex PIN-based browser authentication via `plex login`

Obtaining a Plex token manually is awkward (requires finding it in Plex Web's developer tools or account settings). Plex provides a PIN-based auth flow: POST to `https://plex.tv/api/v2/pins` to get a code, open the browser to `https://app.plex.tv/auth#?clientID=...&code=...`, then poll until the user approves and `authToken` is returned. This is the same flow used by official Plex apps.

The `plex login` subcommand implements this flow, opens the system browser, polls for up to 5 minutes, then saves the token to the config file. It works without requiring existing Radarr or Plex configuration (handled before `AppConfig::load()`).

Alternative considered: OAuth2 flow. Rejected — Plex uses its own PIN-based auth, not standard OAuth2.

### D8: Config file persistence via `init` command and config write-back

Added a `save_to_config()` function that reads the existing TOML config file, merges in provided updates (only overwriting fields that are specified), and writes it back. Config structs now derive both `Serialize` and `Deserialize`.

The `init` command uses this to save Radarr URL, API key, Plex URL, and/or Plex token from CLI flags to the config file. The `plex login` command uses it to save the obtained token. Both commands are handled before `AppConfig::load()` in `main()` so they don't require valid Radarr config to run.

Alternative considered: Separate config files per service. Rejected — single TOML file with sections is simpler and matches existing structure.

## Risks / Trade-offs

- **[Risk] Plex API is undocumented/unstable** → The endpoints we use (`/library/sections`, `/library/sections/{id}/all`) are well-established and used by many third-party tools. Low risk of breakage.
- **[Risk] Large libraries may be slow to fetch** → A single `GET /library/sections/{id}/all` returns all movies in one call. For very large libraries (10k+ movies), this could be slow. Acceptable for now; pagination can be added later if needed.
- **[Risk] GUID matching may not be perfect** → Some movies may have different TMDB IDs in Plex vs Radarr, or missing GUIDs. Unmatched movies are treated as "no watch data" and sorted accordingly (treated as unwatched for Plex strategies).
- **[Trade-off] XML dependency** → Adding `quick-xml` is a new dependency, but it's well-maintained and the only reasonable way to parse Plex responses.
