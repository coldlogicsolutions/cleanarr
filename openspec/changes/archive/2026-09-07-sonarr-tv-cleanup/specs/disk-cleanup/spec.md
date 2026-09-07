## MODIFIED Requirements

### Requirement: Disk usage threshold check
The system SHALL compute the media library size as the sum of Radarr movie `sizeOnDisk` values plus, when Sonarr is configured, the sum of Sonarr series `statistics.sizeOnDisk` values, and determine whether that total exceeds a user-specified threshold.

#### Scenario: Disk usage below threshold
- **WHEN** the user runs `radarrer cleanup 2TB` and the combined library size is at or below 2 TB
- **THEN** the system SHALL report the movie size, TV size (when Sonarr is configured), and threshold, and take no further action

#### Scenario: Disk usage exceeds threshold
- **WHEN** the combined library size exceeds the specified threshold
- **THEN** the system SHALL calculate how much space needs to be freed and proceed to identify candidate movies and episodes for deletion

#### Scenario: Sonarr not configured
- **WHEN** Sonarr is not configured
- **THEN** the system SHALL behave exactly as before: only movie size is counted and only movies are candidates

### Requirement: Deletion strategy selection
The system SHALL support five deletion strategies selectable via the `--strategy` flag: `oldest` (default), `oldest-added`, `never-watched`, `least-watched`, and `oldest-watched`. Strategies SHALL rank movies and episode files together in a single ordered candidate list.

#### Scenario: Oldest strategy (default)
- **WHEN** the user specifies `--strategy oldest` or omits the `--strategy` flag
- **THEN** the system SHALL sort candidates by their effective date ascending, where the effective date is the Plex last watched timestamp if available, otherwise the Radarr `added` date for movies or the Sonarr episode file `dateAdded` for episodes. Candidates are interleaved regardless of type or whether they have Plex data.

#### Scenario: Oldest strategy without Plex
- **WHEN** the user uses the `oldest` strategy and Plex is not configured
- **THEN** the system SHALL sort all candidates by their added date ascending

#### Scenario: Oldest strategy with Plex
- **WHEN** the user uses the `oldest` strategy and Plex is configured
- **THEN** the system SHALL fetch Plex watch data for movies and episodes and sort using last watched date where available, falling back to added date

#### Scenario: Oldest added strategy
- **WHEN** the user specifies `--strategy oldest-added`
- **THEN** the system SHALL sort candidates by added date ascending, ignoring any Plex watch data

#### Scenario: Never-watched strategy
- **WHEN** the user specifies `--strategy never-watched`
- **THEN** the system SHALL select only candidates with a Plex viewCount of 0, sorted by added date ascending

#### Scenario: Least-watched strategy
- **WHEN** the user specifies `--strategy least-watched`
- **THEN** the system SHALL sort candidates by Plex viewCount ascending, then by lastViewedAt ascending

#### Scenario: Oldest-watched strategy
- **WHEN** the user specifies `--strategy oldest-watched`
- **THEN** the system SHALL filter to candidates with viewCount > 0, sort by Plex lastViewedAt ascending

### Requirement: Dry-run mode
The system SHALL default to dry-run mode for the cleanup command. Actual deletion SHALL require an explicit `--confirm` flag.

#### Scenario: Default dry-run
- **WHEN** the user runs `radarrer cleanup 2TB` without `--confirm`
- **THEN** the system SHALL display the movies and episodes that would be deleted, their sizes, and the total space that would be freed, but SHALL NOT delete anything

#### Scenario: Confirmed movie deletion
- **WHEN** the user runs cleanup with `--confirm` and a movie is selected
- **THEN** the system SHALL delete the movie including its files via the Radarr API

#### Scenario: Confirmed episode deletion
- **WHEN** the user runs cleanup with `--confirm` and an episode is selected
- **THEN** the system SHALL delete the episode file via the Sonarr API and then unmonitor the episode so Sonarr does not re-download it

### Requirement: Cleanup output
The system SHALL display a summary of the cleanup operation including: disk space information, movie library size, TV library size (when Sonarr is configured), threshold, space to free, and a table of candidates selected for deletion.

#### Scenario: Cleanup summary display
- **WHEN** the cleanup command identifies candidates for deletion
- **THEN** the system SHALL display a table with columns: Type, ID, Title, Size, Added, and watch columns when Plex is configured, where an episode's Title is rendered as `Series - S02E05 - Episode Title`

#### Scenario: JSON output
- **WHEN** the output format is json
- **THEN** each item SHALL include a `type` field (`movie` or `episode`), and episode items SHALL include `seriesId`, `seriesTitle`, `seasonNumber`, `episodeNumber`, and `episodeFileId`; the summary SHALL include `movieSize`, `tvSize`, and `librarySize`

#### Scenario: Quiet output
- **WHEN** the output format is quiet
- **THEN** the system SHALL print one line per candidate in the form `movie:<id>` or `episode:<episodeId>`

#### Scenario: No candidates to delete
- **WHEN** the cleanup command determines no deletion is needed
- **THEN** the system SHALL display the current disk usage, library sizes, and threshold and indicate no action is needed

### Requirement: Plex section selection for cleanup
The system SHALL support a `--plex-section` flag selecting the Plex movie library and a `--plex-tv-section` flag selecting the Plex show library, each auto-detected when exactly one library of that type exists. A section type is only resolved when candidates of that media type are in scope.

#### Scenario: Auto-detect section
- **WHEN** the user runs cleanup with a Plex-aware strategy and exactly one Plex movie library exists
- **THEN** the system SHALL automatically use that library section for movie watch data

#### Scenario: Specify section
- **WHEN** the user runs `radarrer cleanup 5TB --strategy never-watched --plex-section 2 --plex-tv-section 3`
- **THEN** the system SHALL use section 2 for movie watch data and section 3 for episode watch data

#### Scenario: Multiple sections without flag
- **WHEN** multiple Plex libraries of a needed type exist and the corresponding flag is not given
- **THEN** the system SHALL display an error listing the available libraries of that type and their keys

#### Scenario: TV section not needed
- **WHEN** Sonarr is not configured or `--media movies` is given
- **THEN** the system SHALL NOT resolve or require a Plex show library

### Requirement: Cleanup respects movie exclusions
The system SHALL support a `--exclude-tag` flag that prevents movies and episodes with specified tags from being deleted during cleanup. Tag names are resolved against Radarr tags for movies and against Sonarr tags for series; an episode is excluded when its series carries an excluded tag.

#### Scenario: Exclude tagged movies
- **WHEN** the user runs `radarrer cleanup 2TB --exclude-tag keep`
- **THEN** the system SHALL exclude all movies tagged "keep" in Radarr from the candidate list

#### Scenario: Exclude tagged series
- **WHEN** the user runs `radarrer cleanup 2TB --exclude-tag keep` with Sonarr configured
- **THEN** the system SHALL exclude every episode of any series tagged "keep" in Sonarr from the candidate list

### Requirement: Cleanup progress reporting
The system SHALL report progress during cleanup when deleting multiple candidates.

#### Scenario: Deletion progress
- **WHEN** the cleanup command is deleting candidates with `--confirm`
- **THEN** the system SHALL display progress for each deletion (e.g., `[3/10] Deleted "Show - S01E02 - Title" (4.2 GB freed)`) and a final summary counting movies and episodes deleted

## ADDED Requirements

### Requirement: Media type filter
The system SHALL support a `--media` flag with values `all` (default), `movies`, or `tv` that restricts which candidate types may be deleted. The library size used for the threshold check SHALL always include both movies and TV when Sonarr is configured.

#### Scenario: TV only
- **WHEN** the user runs `radarrer cleanup 2TB --media tv`
- **THEN** the system SHALL count both libraries toward the total but only select episode files for deletion

#### Scenario: TV requested without Sonarr
- **WHEN** the user runs `radarrer cleanup 2TB --media tv` and Sonarr is not configured
- **THEN** the system SHALL display an error indicating Sonarr configuration is required
