## Purpose
Thin Sonarr v3 API client providing the series, episode, tag, and episode-file operations needed by cleanup.

## Requirements

### Requirement: Sonarr API authentication
The system SHALL authenticate with Sonarr using the `X-Api-Key` header on every request and SHALL surface Sonarr error messages using the same format as Radarr errors.

#### Scenario: Invalid API key
- **WHEN** a request is made to Sonarr with an invalid API key
- **THEN** the system SHALL report an API error including the HTTP status and Sonarr's message

### Requirement: List series with disk statistics
The system SHALL fetch all series via `GET /api/v3/series` and expose for each: id, title, tvdbId, tmdbId, imdbId, tags, and `statistics.sizeOnDisk`.

#### Scenario: Fetch series
- **WHEN** the system requests the series list
- **THEN** the system SHALL return typed series records where a missing `statistics` block yields a size of 0

### Requirement: Fetch episodes with file details
The system SHALL fetch episodes for a series via `GET /api/v3/episode?seriesId={id}&includeEpisodeFile=true` and expose id, seasonNumber, episodeNumber, title, hasFile, episodeFileId, monitored, and the embedded episode file's id, size, and dateAdded.

#### Scenario: Episode without file
- **WHEN** an episode has `hasFile: false`
- **THEN** the system SHALL exclude it from cleanup candidates

#### Scenario: Episode with file
- **WHEN** an episode has an embedded `episodeFile` with `size` and `dateAdded`
- **THEN** the system SHALL expose those values for ranking

### Requirement: Delete episode file
The system SHALL delete an episode's file via `DELETE /api/v3/episodefile/{id}`.

#### Scenario: Successful deletion
- **WHEN** the system deletes episode file 42
- **THEN** Sonarr SHALL remove the file from disk and the request SHALL complete without error

### Requirement: Unmonitor episodes
The system SHALL unmonitor episodes via `PUT /api/v3/episode/monitor` with body `{"episodeIds": [...], "monitored": false}`.

#### Scenario: Unmonitor after delete
- **WHEN** an episode file has been deleted during cleanup
- **THEN** the system SHALL unmonitor that episode so Sonarr does not re-download it

### Requirement: List Sonarr tags
The system SHALL fetch tags via `GET /api/v3/tag` returning id and label.

#### Scenario: Resolve tag by label
- **WHEN** the user passes `--exclude-tag keep`
- **THEN** the system SHALL resolve "keep" case-insensitively to a Sonarr tag id
