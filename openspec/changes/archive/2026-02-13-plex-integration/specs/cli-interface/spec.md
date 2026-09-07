## MODIFIED Requirements

### Requirement: Global flags
The CLI SHALL support the following global flags available on all subcommands: `--url <URL>` (Radarr instance URL), `--api-key <KEY>` (API key), `--output <FORMAT>` (table/json/quiet), `--config <PATH>` (config file path override), `--plex-url <URL>` (Plex server URL), `--plex-token <TOKEN>` (Plex authentication token).

#### Scenario: Override config file path
- **WHEN** the user provides `--config /path/to/config.toml`
- **THEN** the system SHALL read configuration from the specified file instead of the default location

#### Scenario: Plex flags with env vars
- **WHEN** the user provides `--plex-url` or `--plex-token` flags or sets `PLEX_URL`/`PLEX_TOKEN` env vars
- **THEN** the system SHALL use those values for Plex API connections, with CLI flags taking priority over env vars over config file
