## ADDED Requirements

### Requirement: Plex library list command
The system SHALL provide a `plex libraries` subcommand that lists all Plex library sections.

#### Scenario: List all libraries
- **WHEN** the user runs `radarrer plex libraries`
- **THEN** the system SHALL display a table with columns: Key, Title, Type for each library section

#### Scenario: JSON output
- **WHEN** the user runs `radarrer plex libraries --output json`
- **THEN** the system SHALL output the library sections as a JSON array

### Requirement: Plex movies command
The system SHALL provide a `plex movies` subcommand that lists movies from a Plex library section with their watch statistics.

#### Scenario: List movies with watch stats
- **WHEN** the user runs `radarrer plex movies`
- **THEN** the system SHALL display a table with columns: Rating Key, Title, Year, View Count, Last Viewed for each movie

#### Scenario: Specify library section
- **WHEN** the user runs `radarrer plex movies --section 1`
- **THEN** the system SHALL fetch movies from the specified library section

#### Scenario: Auto-detect movie library
- **WHEN** the user runs `radarrer plex movies` without `--section` and exactly one movie library exists
- **THEN** the system SHALL automatically use that library section

#### Scenario: Multiple movie libraries without section flag
- **WHEN** the user runs `radarrer plex movies` without `--section` and multiple movie libraries exist
- **THEN** the system SHALL display an error listing the available movie libraries and their keys

### Requirement: Plex watch stats command
The system SHALL provide a `plex stats` subcommand that shows watch statistics for a specific movie, matched by Radarr movie ID.

#### Scenario: Show stats for a movie
- **WHEN** the user runs `radarrer plex stats --movie-id 42`
- **THEN** the system SHALL look up the Radarr movie by ID, find the matching Plex movie via TMDB/IMDB GUID, and display the watch statistics (view count, last viewed date)

#### Scenario: No Plex match found
- **WHEN** the user runs `radarrer plex stats --movie-id 42` and no matching Plex movie is found
- **THEN** the system SHALL display a message indicating the movie was not found in Plex

### Requirement: Plex browser-based login
The system SHALL provide a `plex login` subcommand that authenticates with Plex via the PIN-based browser flow and saves the obtained token to the config file.

#### Scenario: Successful login
- **WHEN** the user runs `radarrer plex login`
- **THEN** the system SHALL create a PIN at plex.tv, open the system browser for the user to authenticate, poll for the auth token (up to 5 minutes), and save the token to the config file

#### Scenario: Login with Plex URL
- **WHEN** the user runs `radarrer plex login --plex-url http://plex:32400`
- **THEN** the system SHALL perform the PIN auth flow and save both the token and the Plex URL to the config file

#### Scenario: Login timeout
- **WHEN** the user runs `radarrer plex login` and does not complete browser authentication within 5 minutes
- **THEN** the system SHALL display a timeout error

#### Scenario: Login without Radarr config
- **WHEN** the user runs `radarrer plex login` without having configured Radarr URL or API key
- **THEN** the system SHALL still proceed with Plex authentication (login does not require Radarr config)

### Requirement: Config init command
The system SHALL provide an `init` top-level command that saves connection settings to the config file.

#### Scenario: Save Radarr connection
- **WHEN** the user runs `radarrer init --url http://radarr:7878 --api-key KEY`
- **THEN** the system SHALL save the URL and API key to the `[connection]` section of the config file

#### Scenario: Save Plex URL
- **WHEN** the user runs `radarrer init --plex-url http://plex:32400`
- **THEN** the system SHALL save the Plex URL to the `[plex]` section of the config file

#### Scenario: Merge with existing config
- **WHEN** the user runs `radarrer init --plex-url http://plex:32400` and a config file already exists with Radarr settings
- **THEN** the system SHALL update only the Plex URL, preserving all existing settings

#### Scenario: No settings provided
- **WHEN** the user runs `radarrer init` without any flags
- **THEN** the system SHALL display an error indicating which flags are available

### Requirement: Plex not configured error
The system SHALL display a clear error when Plex subcommands (other than `login`) are used but Plex is not configured.

#### Scenario: Missing Plex config
- **WHEN** the user runs any `plex` subcommand (except `login`) without having configured Plex URL and token
- **THEN** the system SHALL display an error message indicating Plex is not configured and how to configure it (plex login, CLI flags, env vars, or config file)
