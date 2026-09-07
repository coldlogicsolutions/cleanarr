## ADDED Requirements

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
