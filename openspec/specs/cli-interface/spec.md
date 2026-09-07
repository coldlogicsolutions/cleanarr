## Purpose
Command-line surfaces of the two binaries: `cleanarr` (cleanup, plex, init, completions) and `radarr-cli` (one subcommand per Radarr API resource group). Both share output formats and shell completion. Radarr resource subcommands in this spec refer to `radarr-cli`; `cleanarr` exposes none of them.

## Requirements

### Requirement: Subcommand structure mirrors API resource groups
The `radarr-cli` binary SHALL provide top-level subcommands for each major Radarr API resource group. Each subcommand SHALL have sub-subcommands for the available operations (list, get, create, update, delete, etc.).

#### Scenario: Movie CRUD operations
- **WHEN** the user runs `radarr-cli movie list`
- **THEN** the system SHALL call `GET /api/v3/movie` and display all movies

#### Scenario: Movie lookup
- **WHEN** the user runs `radarr-cli movie lookup --term "The Matrix"`
- **THEN** the system SHALL call `GET /api/v3/movie/lookup?term=The+Matrix` and display search results

#### Scenario: Get single movie
- **WHEN** the user runs `radarr-cli movie get <id>`
- **THEN** the system SHALL call `GET /api/v3/movie/{id}` and display the movie details

#### Scenario: Delete movie
- **WHEN** the user runs `radarr-cli movie delete <id> --delete-files`
- **THEN** the system SHALL call `DELETE /api/v3/movie/{id}?deleteFiles=true`

### Requirement: Full API coverage
The `radarr-cli` binary SHALL provide subcommands for all Radarr v3 API resource groups including but not limited to: movie, moviefile, queue, history, calendar, collection, command, system (health, diskspace, backup, status, task, update), config (host, ui, naming, mediamanagement, downloadclient, indexer, importlist, metadata), qualityprofile, qualitydefinition, customformat, delayprofile, indexer, downloadclient, notification, importlist, tag, blocklist, exclusions, release, releaseprofile, remotepathmapping, parse, rename, manualimport, credit, alttitle, autotagging, customfilter, log, and extrafile.

#### Scenario: System health check
- **WHEN** the user runs `radarr-cli system health`
- **THEN** the system SHALL call `GET /api/v3/health` and display health check results

#### Scenario: Queue management
- **WHEN** the user runs `radarr-cli queue list`
- **THEN** the system SHALL call `GET /api/v3/queue` and display queued downloads

#### Scenario: History browsing
- **WHEN** the user runs `radarr-cli history list --page 1 --page-size 20`
- **THEN** the system SHALL call `GET /api/v3/history?page=1&pageSize=20` and display results

#### Scenario: Trigger command
- **WHEN** the user runs `radarr-cli command run RefreshMovie --movieIds 1,2,3`
- **THEN** the system SHALL call `POST /api/v3/command` with the appropriate command body

### Requirement: Output format selection
The system SHALL support three output formats selectable via the `--output` flag or config file: `table` (default), `json`, and `quiet`.

#### Scenario: Default table output
- **WHEN** no `--output` flag is provided
- **THEN** the system SHALL display results as a human-readable table with column headers

#### Scenario: JSON output
- **WHEN** the user provides `--output json`
- **THEN** the system SHALL output the raw API response as pretty-printed JSON

#### Scenario: Quiet output
- **WHEN** the user provides `--output quiet`
- **THEN** the system SHALL output only essential identifiers (IDs, counts) with no headers or formatting, suitable for piping to other commands

### Requirement: Global flags
The `cleanarr` binary SHALL support the following global flags available on all subcommands: `--radarr-url <URL>` (Radarr instance URL), `--radarr-api-key <KEY>` (Radarr API key), `--output <FORMAT>` (table/json/quiet), `--config <PATH>` (config file path override), `--plex-url <URL>` (Plex server URL), `--plex-token <TOKEN>` (Plex authentication token), `--sonarr-url <URL>` (Sonarr instance URL), `--sonarr-api-key <KEY>` (Sonarr API key). The `radarr-cli` binary SHALL support `--url <URL>` and `--api-key <KEY>` (with `--radarr-url` and `--radarr-api-key` accepted as aliases for compatibility with `cleanarr`), `--output <FORMAT>`, and `--config <PATH>` (default `~/.config/radarr-cli/config.toml`), with `RADARR_URL`, `RADARR_API_KEY`, and `RADARR_OUTPUT` as environment fallbacks.

#### Scenario: Override config file path
- **WHEN** the user provides `--config /path/to/config.toml`
- **THEN** the system SHALL read configuration from the specified file instead of the default location

#### Scenario: Plex flags with env vars
- **WHEN** the user provides `--plex-url` or `--plex-token` flags or sets `PLEX_URL`/`PLEX_TOKEN` env vars
- **THEN** the system SHALL use those values for Plex API connections, with CLI flags taking priority over env vars over config file

#### Scenario: Sonarr flags with env vars
- **WHEN** the user provides `--sonarr-url` or `--sonarr-api-key` flags, sets `SONARR_URL`/`SONARR_API_KEY` env vars, or has a `[sonarr]` section with `url` and `api_key` in the config file
- **THEN** the system SHALL use those values for Sonarr API connections, with CLI flags taking priority over env vars over config file, and Sonarr SHALL be treated as unconfigured unless both url and api key are present

#### Scenario: Init saves Sonarr settings
- **WHEN** the user runs `cleanarr init --sonarr-url http://localhost:8989 --sonarr-api-key KEY`
- **THEN** the system SHALL persist both values under the `[sonarr]` section of the config file

### Requirement: Pagination support
For paginated API endpoints, the CLI SHALL support `--page` and `--page-size` flags to control pagination, and `--sort-key` and `--sort-dir` flags for sorting.

#### Scenario: Custom page size
- **WHEN** the user runs `radarr-cli history list --page-size 50`
- **THEN** the system SHALL request 50 records per page from the API

#### Scenario: Sort by field
- **WHEN** the user runs `radarr-cli movie list --sort-key sortTitle --sort-dir ascending`
- **THEN** the system SHALL pass the sort parameters to the API

### Requirement: Shell completion generation
The system SHALL provide a `completions` subcommand that generates shell completion scripts for bash, zsh, fish, and powershell.

#### Scenario: Generate bash completions
- **WHEN** the user runs `cleanarr completions bash` or `radarr-cli completions bash`
- **THEN** the system SHALL output a bash completion script to stdout

### Requirement: Bulk operations
For API endpoints that support bulk operations, the CLI SHALL provide corresponding bulk subcommands that accept multiple IDs.

#### Scenario: Bulk delete from blocklist
- **WHEN** the user runs `radarr-cli blocklist delete --ids 1,2,3`
- **THEN** the system SHALL call `DELETE /api/v3/blocklist/bulk` with the specified IDs

#### Scenario: Bulk edit movies
- **WHEN** the user runs `radarr-cli movie edit --ids 1,2,3 --monitored true`
- **THEN** the system SHALL call `PUT /api/v3/movie/editor` with the specified changes

### Requirement: Protected tags configuration
The system SHALL read a `protected_tags` string array from the `[cleanup]` section of the config file and a comma-separated `CLEANARR_PROTECTED_TAGS` environment variable, and SHALL persist the list via `cleanarr init --protected-tags <a,b,...>`.

#### Scenario: Init saves protected tags
- **WHEN** the user runs `cleanarr init --protected-tags keep,kids`
- **THEN** the config file SHALL contain `protected_tags = ["keep", "kids"]` under `[cleanup]`, replacing any previous list

#### Scenario: Env var parsed
- **WHEN** `CLEANARR_PROTECTED_TAGS=keep, kids` is set
- **THEN** the system SHALL treat "keep" and "kids" (trimmed, empty entries dropped) as protected

### Requirement: Cleanup defaults configuration
The system SHALL persist cleanup defaults via `cleanarr init --default-max-size <SIZE>` and `--default-strategy <STRATEGY>` under `[cleanup]`, and SHALL honour `CLEANARR_CLEANUP_MAX_SIZE` and `CLEANARR_CLEANUP_STRATEGY` environment variables.

#### Scenario: Init saves defaults
- **WHEN** the user runs `cleanarr init --default-max-size 4TB --default-strategy oldest-added`
- **THEN** the config file SHALL contain both values under `[cleanup]`
