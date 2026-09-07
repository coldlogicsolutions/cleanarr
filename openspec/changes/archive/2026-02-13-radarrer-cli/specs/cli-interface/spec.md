## ADDED Requirements

### Requirement: Subcommand structure mirrors API resource groups
The CLI SHALL provide top-level subcommands for each major Radarr API resource group. Each subcommand SHALL have sub-subcommands for the available operations (list, get, create, update, delete, etc.).

#### Scenario: Movie CRUD operations
- **WHEN** the user runs `radarrer movie list`
- **THEN** the system SHALL call `GET /api/v3/movie` and display all movies

#### Scenario: Movie lookup
- **WHEN** the user runs `radarrer movie lookup --term "The Matrix"`
- **THEN** the system SHALL call `GET /api/v3/movie/lookup?term=The+Matrix` and display search results

#### Scenario: Get single movie
- **WHEN** the user runs `radarrer movie get <id>`
- **THEN** the system SHALL call `GET /api/v3/movie/{id}` and display the movie details

#### Scenario: Delete movie
- **WHEN** the user runs `radarrer movie delete <id> --delete-files`
- **THEN** the system SHALL call `DELETE /api/v3/movie/{id}?deleteFiles=true`

### Requirement: Full API coverage
The CLI SHALL provide subcommands for all Radarr v3 API resource groups including but not limited to: movie, moviefile, queue, history, calendar, collection, command, system (health, diskspace, backup, status, task, update), config (host, ui, naming, mediamanagement, downloadclient, indexer, importlist, metadata), qualityprofile, qualitydefinition, customformat, delayprofile, indexer, downloadclient, notification, importlist, tag, blocklist, exclusions, release, releaseprofile, remotepathmapping, parse, rename, manualimport, credit, alttitle, autotagging, customfilter, log, and extrafile.

#### Scenario: System health check
- **WHEN** the user runs `radarrer system health`
- **THEN** the system SHALL call `GET /api/v3/health` and display health check results

#### Scenario: Queue management
- **WHEN** the user runs `radarrer queue list`
- **THEN** the system SHALL call `GET /api/v3/queue` and display queued downloads

#### Scenario: History browsing
- **WHEN** the user runs `radarrer history list --page 1 --page-size 20`
- **THEN** the system SHALL call `GET /api/v3/history?page=1&pageSize=20` and display results

#### Scenario: Trigger command
- **WHEN** the user runs `radarrer command run RefreshMovie --movieIds 1,2,3`
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
The CLI SHALL support the following global flags available on all subcommands: `--url <URL>` (Radarr instance URL), `--api-key <KEY>` (API key), `--output <FORMAT>` (table/json/quiet), `--config <PATH>` (config file path override).

#### Scenario: Override config file path
- **WHEN** the user provides `--config /path/to/config.toml`
- **THEN** the system SHALL read configuration from the specified file instead of the default location

### Requirement: Pagination support
For paginated API endpoints, the CLI SHALL support `--page` and `--page-size` flags to control pagination, and `--sort-key` and `--sort-dir` flags for sorting.

#### Scenario: Custom page size
- **WHEN** the user runs `radarrer history list --page-size 50`
- **THEN** the system SHALL request 50 records per page from the API

#### Scenario: Sort by field
- **WHEN** the user runs `radarrer movie list --sort-key sortTitle --sort-dir ascending`
- **THEN** the system SHALL pass the sort parameters to the API

### Requirement: Shell completion generation
The system SHALL provide a `completions` subcommand that generates shell completion scripts for bash, zsh, fish, and powershell.

#### Scenario: Generate bash completions
- **WHEN** the user runs `radarrer completions bash`
- **THEN** the system SHALL output a bash completion script to stdout

### Requirement: Bulk operations
For API endpoints that support bulk operations, the CLI SHALL provide corresponding bulk subcommands that accept multiple IDs.

#### Scenario: Bulk delete from blocklist
- **WHEN** the user runs `radarrer blocklist delete --ids 1,2,3`
- **THEN** the system SHALL call `DELETE /api/v3/blocklist/bulk` with the specified IDs

#### Scenario: Bulk edit movies
- **WHEN** the user runs `radarrer movie edit --ids 1,2,3 --monitored true`
- **THEN** the system SHALL call `PUT /api/v3/movie/editor` with the specified changes
