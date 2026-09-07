## MODIFIED Requirements

### Requirement: Global flags
The CLI SHALL support the following global flags available on all subcommands: `--url <URL>` (Radarr instance URL), `--api-key <KEY>` (API key), `--output <FORMAT>` (table/json/quiet), `--config <PATH>` (config file path override), `--plex-url <URL>` (Plex server URL), `--plex-token <TOKEN>` (Plex authentication token), `--sonarr-url <URL>` (Sonarr instance URL), `--sonarr-api-key <KEY>` (Sonarr API key).

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
- **WHEN** the user runs `radarrer init --sonarr-url http://localhost:8989 --sonarr-api-key KEY`
- **THEN** the system SHALL persist both values under the `[sonarr]` section of the config file
