## Purpose
Read-only Plex Media Server client for library sections, movies, shows, and episodes with watch statistics and external ID matching.

## Requirements

### Requirement: Plex API authentication
The system SHALL authenticate with the Plex Media Server using an `X-Plex-Token` query parameter appended to all API requests.

#### Scenario: Valid token
- **WHEN** a request is made to the Plex API with a valid token
- **THEN** the system SHALL receive a 200 response with the requested data

#### Scenario: Invalid token
- **WHEN** a request is made to the Plex API with an invalid token
- **THEN** the system SHALL report an authentication error with a clear message indicating the Plex token is invalid

### Requirement: List library sections
The system SHALL fetch all library sections from the Plex server via `GET /library/sections` and return typed section data including the section key, title, and type.

#### Scenario: Fetch libraries
- **WHEN** the system requests library sections from Plex
- **THEN** the system SHALL return a list of library sections with their key (numeric ID), title, and type (e.g., "movie", "show")

#### Scenario: Filter movie libraries
- **WHEN** the system needs to identify movie libraries
- **THEN** the system SHALL filter sections to only those with `type="movie"`

### Requirement: Fetch movies with watch statistics
The system SHALL fetch all movies from a Plex library section via `GET /library/sections/{id}/all` and return typed movie data including watch statistics.

#### Scenario: Fetch all movies from section
- **WHEN** the system requests all movies from a Plex library section
- **THEN** the system SHALL return a list of movies with at minimum: ratingKey, title, year, viewCount (defaulting to 0 if absent), lastViewedAt (as Option), addedAt, and external GUIDs

#### Scenario: Movie never watched
- **WHEN** a movie has never been watched on Plex
- **THEN** the system SHALL report viewCount as 0 and lastViewedAt as None

### Requirement: Extract external GUIDs from Plex movies
The system SHALL parse the Guid elements on each Plex movie to extract TMDB and IMDB identifiers for matching against Radarr movies.

#### Scenario: Movie with TMDB GUID
- **WHEN** a Plex movie has a Guid element with id `tmdb://12345`
- **THEN** the system SHALL extract the TMDB ID as 12345

#### Scenario: Movie with IMDB GUID
- **WHEN** a Plex movie has a Guid element with id `imdb://tt1234567`
- **THEN** the system SHALL extract the IMDB ID as "tt1234567"

#### Scenario: Movie with no matching GUIDs
- **WHEN** a Plex movie has no TMDB or IMDB GUIDs
- **THEN** the system SHALL treat the movie as unmatched (no watch data available for Radarr matching)

### Requirement: XML response deserialization
The system SHALL deserialize Plex XML responses into typed Rust structs using serde-compatible XML deserialization.

#### Scenario: Parse library sections response
- **WHEN** the Plex API returns an XML MediaContainer with Directory elements
- **THEN** the system SHALL deserialize them into a Vec of typed section structs

#### Scenario: Parse movies response
- **WHEN** the Plex API returns an XML MediaContainer with Video elements
- **THEN** the system SHALL deserialize them into a Vec of typed movie structs with all relevant fields

### Requirement: Plex PIN-based authentication
The system SHALL support authenticating with Plex via the plex.tv PIN-based flow for obtaining an auth token without manual token retrieval.

#### Scenario: Create PIN
- **WHEN** the system initiates Plex PIN authentication
- **THEN** the system SHALL POST to `https://plex.tv/api/v2/pins` with `X-Plex-Product: Cleanarr` and a per-install `X-Plex-Client-Identifier` (a UUID generated on first login and stored as `client_id` under `[plex]` in the config file, reused on later logins), and receive a PIN ID and code

#### Scenario: Poll for auth token
- **WHEN** the system has a PIN ID and the user has authenticated in the browser
- **THEN** the system SHALL poll `GET https://plex.tv/api/v2/pins/{id}` until `authToken` is non-empty in the response

#### Scenario: Save obtained token
- **WHEN** the PIN auth flow completes successfully
- **THEN** the system SHALL persist the obtained auth token to the config file's `[plex]` section

### Requirement: Config file write-back
The system SHALL support writing connection settings back to the TOML config file, merging updates with existing configuration without losing unmodified settings.

#### Scenario: Update single field
- **WHEN** the system saves a new Plex token to config
- **THEN** the system SHALL read the existing config, update only the `[plex].token` field, and write back the full config preserving all other settings

#### Scenario: Create config directory
- **WHEN** the config directory does not exist
- **THEN** the system SHALL create the directory structure before writing the config file

### Requirement: Plex configuration
The system SHALL support Plex connection configuration via CLI flags (`--plex-url`, `--plex-token`), environment variables (`PLEX_URL`, `PLEX_TOKEN`), and config file (`[plex]` section with `url` and `token` fields), following the same layering as Radarr config: CLI flags > env vars > config file.

#### Scenario: Config from environment
- **WHEN** `PLEX_URL` and `PLEX_TOKEN` environment variables are set
- **THEN** the system SHALL use those values for Plex API connections

#### Scenario: Config from file
- **WHEN** the config file contains a `[plex]` section with `url` and `token`
- **THEN** the system SHALL use those values for Plex API connections when no CLI flags or env vars override them

#### Scenario: Plex not configured
- **WHEN** no Plex URL or token is provided via any configuration source
- **THEN** the Plex client SHALL not be initialized and Plex-dependent features SHALL gracefully degrade

### Requirement: Fetch TV shows with external GUIDs
The system SHALL fetch all shows from a Plex `show` library section via `GET /library/sections/{id}/all?type=2&includeGuids=1` and expose ratingKey, title, and TVDB/TMDB/IMDB identifiers parsed from Guid children.

#### Scenario: Show with TVDB GUID
- **WHEN** a Plex show has a Guid element with id `tvdb://81189`
- **THEN** the system SHALL extract the TVDB ID as 81189

#### Scenario: Show with no external GUIDs
- **WHEN** a Plex show has no TVDB, TMDB, or IMDB GUIDs
- **THEN** the system SHALL treat all of its episodes as unmatched

### Requirement: Fetch episodes with watch statistics
The system SHALL fetch all episodes from a Plex `show` library section via `GET /library/sections/{id}/all?type=4` and expose ratingKey, grandparentRatingKey (show), parentIndex (season number), index (episode number), viewCount (default 0), and lastViewedAt.

#### Scenario: Fetch episodes
- **WHEN** the system requests episodes for section 3
- **THEN** the system SHALL return one record per episode with its show key, season and episode numbers, and watch statistics

### Requirement: Resolve TV library section
The system SHALL resolve a Plex `show` section from `--plex-tv-section`, auto-detecting when exactly one show library exists.

#### Scenario: Single show library
- **WHEN** no `--plex-tv-section` is given and exactly one section has type `show`
- **THEN** the system SHALL use that section

#### Scenario: Multiple show libraries
- **WHEN** no `--plex-tv-section` is given and more than one section has type `show`
- **THEN** the system SHALL display an error listing the show libraries and their keys

### Requirement: Match Sonarr episodes to Plex episodes
The system SHALL match a Sonarr episode to a Plex episode by first matching the series (TVDB ID, then TMDB ID, then IMDB ID) to a Plex show, then matching season number and episode number.

#### Scenario: Matched episode
- **WHEN** Sonarr series tvdbId 81189 season 2 episode 5 exists and Plex has a show with `tvdb://81189` containing an episode with parentIndex 2 and index 5
- **THEN** the system SHALL attach that Plex episode's viewCount and lastViewedAt to the Sonarr episode

#### Scenario: Unmatched episode
- **WHEN** no Plex show matches the series or no episode matches the season/episode numbers
- **THEN** the system SHALL leave the episode without watch data
