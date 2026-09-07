## Purpose
The `cleanup` command: compare combined movie and TV library size against a threshold and delete movies or episode files by strategy.

## Requirements

### Requirement: Disk usage threshold check
The system SHALL compute the media library size as the sum of Radarr movie `sizeOnDisk` values plus, when Sonarr is configured, the sum of Sonarr series `statistics.sizeOnDisk` values, and determine whether that total exceeds a user-specified threshold.

#### Scenario: Disk usage below threshold
- **WHEN** the user runs `cleanarr cleanup 2TB` and the combined library size is at or below 2 TB
- **THEN** the system SHALL report the movie size, TV size (when Sonarr is configured), and threshold, and take no further action

#### Scenario: Disk usage exceeds threshold
- **WHEN** the combined library size exceeds the specified threshold
- **THEN** the system SHALL calculate how much space needs to be freed and proceed to identify candidate movies and episodes for deletion

#### Scenario: Sonarr not configured
- **WHEN** Sonarr is not configured
- **THEN** the system SHALL behave exactly as before: only movie size is counted and only movies are candidates

### Requirement: Size threshold format
The system SHALL accept the size threshold either as the positional `MAX_SIZE` argument or via the `--max-size <SIZE>` flag (mutually exclusive), with human-readable values supporting units: B, KB, MB, GB, TB (decimal/SI).

#### Scenario: Terabyte threshold
- **WHEN** the user specifies `cleanarr cleanup 2TB` or `cleanarr cleanup --max-size 2TB`
- **THEN** the system SHALL interpret this as 2,000,000,000,000 bytes (decimal/SI units)

#### Scenario: Gigabyte threshold
- **WHEN** the user specifies `--max-size 500GB`
- **THEN** the system SHALL interpret this as 500,000,000,000 bytes

#### Scenario: Both forms given
- **WHEN** the user passes both a positional size and `--max-size`
- **THEN** the system SHALL reject the invocation with a usage error

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

#### Scenario: Oldest strategy when a Plex library cannot be resolved
- **WHEN** the strategy is `oldest` and the Plex movie or show library cannot be resolved or fetched (e.g. no show library, or several without `--plex-tv-section`)
- **THEN** the system SHALL print a warning and continue without watch data for that media type instead of aborting; Plex-dependent strategies keep the hard error

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
- **WHEN** the user runs `cleanarr cleanup 2TB` without `--confirm`
- **THEN** the system SHALL display the movies and episodes that would be deleted, their sizes, and the total space that would be freed, but SHALL NOT delete anything

#### Scenario: Confirmed movie deletion
- **WHEN** the user runs cleanup with `--confirm` and a movie is selected
- **THEN** the system SHALL delete the movie including its files via the Radarr API

#### Scenario: Confirmed episode deletion
- **WHEN** the user runs cleanup with `--confirm` and an episode is selected
- **THEN** the system SHALL delete the episode file via the Sonarr API and then unmonitor the episode so Sonarr does not re-download it

#### Scenario: Multi-episode file
- **WHEN** several Sonarr episodes share one episode file (e.g. S01E01E02)
- **THEN** the file SHALL appear as a single candidate whose size is counted once and whose title lists the episode range, one DELETE SHALL be issued, and every episode sharing the file SHALL be unmonitored

### Requirement: Cleanup output
The system SHALL display a summary of the cleanup operation including: disk space information, movie library size, TV library size (when Sonarr is configured), threshold, space to free, and a table of candidates selected for deletion.

#### Scenario: Cleanup summary display
- **WHEN** the cleanup command identifies candidates for deletion
- **THEN** the system SHALL display a table with columns: Type, ID, Title, Size, Added, and watch columns when Plex is configured, where an episode's Title is rendered as `Series - S02E05 - Episode Title`

#### Scenario: JSON output
- **WHEN** the output format is json
- **THEN** each item SHALL include a `type` field (`movie` or `episode`) and a `status` field (`planned` without `--confirm`; `deleted` or `failed` with an `error` string after a confirmed run); episode items SHALL include `seriesId`, `seriesTitle`, `seasonNumber`, `episodeNumber`, `episodeNumbers`, `episodeIds`, and `episodeFileId`; the summary SHALL include `movieSize`, `tvSize`, `librarySize`, and `unmatched` (count of candidates with no Plex data)

#### Scenario: JSON output after a confirmed run
- **WHEN** the output format is json and `--confirm` is given
- **THEN** the system SHALL emit the summary document after the deletion loop, with per-item `status`, and top-level `deleted`, `failed`, and `freed` (bytes actually freed)

#### Scenario: Quiet output
- **WHEN** the output format is quiet
- **THEN** the system SHALL print one line per candidate in the form `movie:<id>` or `episode:<episodeId>` regardless of whether Sonarr is configured or `--media` is given; with `--confirm`, only successfully deleted candidates are printed

#### Scenario: No candidates to delete
- **WHEN** the cleanup command determines no deletion is needed
- **THEN** the system SHALL display the current disk usage, library sizes, and threshold and indicate no action is needed

### Requirement: Never-watched cleanup strategy
The system SHALL support a `never-watched` cleanup strategy that prioritizes deleting candidates (movies and episodes) that have never been played on Plex. Candidates that could not be matched to a Plex item have unknown watch state and SHALL be excluded from this strategy, with their count reported.

#### Scenario: Never-watched strategy selection
- **WHEN** the user runs `cleanarr cleanup 5TB --strategy never-watched`
- **THEN** the system SHALL select only candidates with a Plex viewCount of 0, sorted by added date ascending (oldest added first)

#### Scenario: Candidates not found in Plex
- **WHEN** some candidates have no matching Plex item
- **THEN** the system SHALL NOT treat them as unwatched; it SHALL skip them for this strategy and report `N candidates not found in Plex`

#### Scenario: All movies watched
- **WHEN** the user runs `cleanarr cleanup 5TB --strategy never-watched` and all movies have been watched at least once
- **THEN** the system SHALL report that no unwatched candidates are available for cleanup

#### Scenario: Never-watched requires Plex
- **WHEN** the user runs `cleanarr cleanup 5TB --strategy never-watched` without Plex configured
- **THEN** the system SHALL display an error indicating that the never-watched strategy requires Plex configuration

### Requirement: Least-watched cleanup strategy
The system SHALL support a `least-watched` cleanup strategy that prioritizes deleting candidates (movies and episodes) with the lowest play count, using last viewed date as a tiebreaker. Candidates without Plex watch data SHALL be excluded.

#### Scenario: Least-watched strategy sorting
- **WHEN** the user runs `cleanarr cleanup 5TB --strategy least-watched`
- **THEN** the system SHALL sort matched candidates by Plex viewCount ascending, then by lastViewedAt ascending (least recently watched first among ties), and select candidates for deletion from the top

#### Scenario: Least-watched requires Plex
- **WHEN** the user runs `cleanarr cleanup 5TB --strategy least-watched` without Plex configured
- **THEN** the system SHALL display an error indicating that the least-watched strategy requires Plex configuration

### Requirement: Oldest-watched cleanup strategy
The system SHALL support an `oldest-watched` cleanup strategy that prioritizes deleting candidates (movies and episodes) that were watched longest ago, excluding candidates that have never been watched or have no Plex data.

#### Scenario: Oldest-watched strategy sorting
- **WHEN** the user runs `cleanarr cleanup 5TB --strategy oldest-watched`
- **THEN** the system SHALL filter to only candidates with viewCount > 0, sort by Plex lastViewedAt ascending (oldest watch date first), and select candidates for deletion from the top

#### Scenario: Oldest-watched excludes unwatched
- **WHEN** the user runs `cleanarr cleanup 5TB --strategy oldest-watched` and some movies have never been watched
- **THEN** the system SHALL exclude candidates with viewCount of 0 from the candidate list

#### Scenario: Oldest-watched requires Plex
- **WHEN** the user runs `cleanarr cleanup 5TB --strategy oldest-watched` without Plex configured
- **THEN** the system SHALL display an error indicating that the oldest-watched strategy requires Plex configuration

### Requirement: Plex watch data in cleanup output
The system SHALL include Plex watch statistics in the cleanup output table when Plex is configured.

#### Scenario: Table output with Plex data
- **WHEN** the cleanup command identifies movies for deletion and Plex is configured
- **THEN** the system SHALL display View Count and Last Watched columns populated from Plex data alongside the existing columns

#### Scenario: Table output without Plex
- **WHEN** the cleanup command identifies movies for deletion and Plex is not configured
- **THEN** the system SHALL display the existing columns without Plex watch data (backwards compatible)

### Requirement: Plex section selection for cleanup
The system SHALL support a `--plex-section` flag selecting the Plex movie library and a `--plex-tv-section` flag selecting the Plex show library, each auto-detected when exactly one library of that type exists. A section type is only resolved when candidates of that media type are in scope.

#### Scenario: Auto-detect section
- **WHEN** the user runs cleanup with a Plex-aware strategy and exactly one Plex movie library exists
- **THEN** the system SHALL automatically use that library section for movie watch data

#### Scenario: Specify section
- **WHEN** the user runs `cleanarr cleanup 5TB --strategy never-watched --plex-section 2 --plex-tv-section 3`
- **THEN** the system SHALL use section 2 for movie watch data and section 3 for episode watch data

#### Scenario: Multiple sections without flag
- **WHEN** multiple Plex libraries of a needed type exist and the corresponding flag is not given
- **THEN** the system SHALL display an error listing the available libraries of that type and their keys

#### Scenario: TV section not needed
- **WHEN** Sonarr is not configured or `--media movies` is given
- **THEN** the system SHALL NOT resolve or require a Plex show library

### Requirement: Cleanup respects movie exclusions
The system SHALL never delete movies or episodes carrying a protected tag. The protected set is the union of `protected_tags` in the `[cleanup]` config section, the comma-separated `CLEANARR_PROTECTED_TAGS` env var, and `--exclude-tag` values. Tag names are resolved case-insensitively against Radarr tags for movies and Sonarr tags for series; an episode is protected when its series carries a protected tag.

#### Scenario: Exclude tagged movies
- **WHEN** the user runs `cleanarr cleanup 2TB --exclude-tag keep`
- **THEN** the system SHALL exclude all movies tagged "keep" in Radarr from the candidate list

#### Scenario: Exclude tagged series
- **WHEN** the user runs `cleanarr cleanup 2TB --exclude-tag keep` with Sonarr configured
- **THEN** the system SHALL exclude every episode of any series tagged "keep" in Sonarr from the candidate list

#### Scenario: Configured protected tags
- **WHEN** the config file contains `[cleanup] protected_tags = ["keep"]` and the user runs `cleanarr cleanup 2TB` with no `--exclude-tag`
- **THEN** the system SHALL exclude movies and series tagged "keep" exactly as if `--exclude-tag keep` had been given

#### Scenario: CLI flag is additive
- **WHEN** the config file protects "keep" and the user passes `--exclude-tag kids`
- **THEN** the system SHALL protect both "keep" and "kids"

#### Scenario: Protected tags shown in output
- **WHEN** any protected tags are in effect and the output format is table
- **THEN** the system SHALL print the effective protected tag names before the candidate table

#### Scenario: Tag names are normalised
- **WHEN** the user passes `--exclude-tag "keep, kids"`
- **THEN** the system SHALL trim whitespace so both `keep` and `kids` are protected, exactly as `CLEANARR_PROTECTED_TAGS="keep,kids"` would be

#### Scenario: Protected tag not found in a service
- **WHEN** a protected tag name matches no tag in Radarr (with movie candidates in scope) or no tag in Sonarr (with TV candidates in scope)
- **THEN** the system SHALL print a warning naming the tag and the service, and with `--confirm` SHALL refuse to delete unless `--allow-unresolved-tags` is given

### Requirement: Cleanup progress reporting
The system SHALL report progress during cleanup when deleting multiple candidates.

#### Scenario: Deletion progress
- **WHEN** the cleanup command is deleting candidates with `--confirm`
- **THEN** the system SHALL display progress for each deletion (e.g., `[3/10] Deleted "Show - S01E02 - Title" (4.2 GB freed)`) and a final summary counting movies and episodes deleted

#### Scenario: Deletion failure
- **WHEN** deleting one candidate fails (e.g. Sonarr returns 500 for an episode file)
- **THEN** the system SHALL report the failing item with its type, ids, title, and the API error, continue with the remaining candidates, count only successful deletions in the summary, and exit non-zero listing all failures

### Requirement: Media type filter
The system SHALL support a `--media` flag with values `all` (default), `movies`, or `tv` that restricts which candidate types may be deleted. The library size used for the threshold check SHALL always include both movies and TV when Sonarr is configured.

#### Scenario: TV only
- **WHEN** the user runs `cleanarr cleanup 2TB --media tv`
- **THEN** the system SHALL count both libraries toward the total but only select episode files for deletion

#### Scenario: TV requested without Sonarr
- **WHEN** the user runs `cleanarr cleanup 2TB --media tv` and Sonarr is not configured
- **THEN** the system SHALL display an error indicating Sonarr configuration is required

### Requirement: Cleanup defaults from configuration
The system SHALL resolve the size threshold and strategy as CLI argument, then `CLEANARR_CLEANUP_MAX_SIZE` / `CLEANARR_CLEANUP_STRATEGY` environment variables, then `default_max_size` / `default_strategy` in the `[cleanup]` config section. Strategy SHALL fall back to `oldest`; a missing threshold SHALL be an error.

#### Scenario: Threshold from config
- **WHEN** the config file has `default_max_size = "4TB"` and the user runs `cleanarr cleanup`
- **THEN** the system SHALL use 4 TB as the threshold

#### Scenario: CLI overrides config
- **WHEN** the config file has `default_max_size = "4TB"` and the user runs `cleanarr cleanup 2TB`
- **THEN** the system SHALL use 2 TB

#### Scenario: Strategy from config
- **WHEN** the config file has `default_strategy = "never-watched"` and no `--strategy` is given
- **THEN** the system SHALL use the `never-watched` strategy, validating it like a CLI value

#### Scenario: No threshold anywhere
- **WHEN** no threshold is given on the CLI, in the environment, or in the config file
- **THEN** the system SHALL exit with an error naming all three ways to provide one
