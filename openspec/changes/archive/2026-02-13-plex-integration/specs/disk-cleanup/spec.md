## ADDED Requirements

### Requirement: Never-watched cleanup strategy
The system SHALL support a `never-watched` cleanup strategy that prioritizes deleting movies that have never been played on Plex.

#### Scenario: Never-watched strategy selection
- **WHEN** the user runs `radarrer cleanup 5TB --strategy never-watched`
- **THEN** the system SHALL select only movies with a Plex viewCount of 0, sorted by added date ascending (oldest added first)

#### Scenario: All movies watched
- **WHEN** the user runs `radarrer cleanup 5TB --strategy never-watched` and all movies have been watched at least once
- **THEN** the system SHALL report that no unwatched movies are available for cleanup

#### Scenario: Never-watched requires Plex
- **WHEN** the user runs `radarrer cleanup 5TB --strategy never-watched` without Plex configured
- **THEN** the system SHALL display an error indicating that the never-watched strategy requires Plex configuration

### Requirement: Least-watched cleanup strategy
The system SHALL support a `least-watched` cleanup strategy that prioritizes deleting movies with the lowest play count, using last viewed date as a tiebreaker.

#### Scenario: Least-watched strategy sorting
- **WHEN** the user runs `radarrer cleanup 5TB --strategy least-watched`
- **THEN** the system SHALL sort movies by Plex viewCount ascending, then by lastViewedAt ascending (least recently watched first among ties), and select movies for deletion from the top

#### Scenario: Least-watched requires Plex
- **WHEN** the user runs `radarrer cleanup 5TB --strategy least-watched` without Plex configured
- **THEN** the system SHALL display an error indicating that the least-watched strategy requires Plex configuration

### Requirement: Oldest-watched cleanup strategy
The system SHALL support an `oldest-watched` cleanup strategy that prioritizes deleting movies that were watched longest ago, excluding movies that have never been watched.

#### Scenario: Oldest-watched strategy sorting
- **WHEN** the user runs `radarrer cleanup 5TB --strategy oldest-watched`
- **THEN** the system SHALL filter to only movies with viewCount > 0, sort by Plex lastViewedAt ascending (oldest watch date first), and select movies for deletion from the top

#### Scenario: Oldest-watched excludes unwatched
- **WHEN** the user runs `radarrer cleanup 5TB --strategy oldest-watched` and some movies have never been watched
- **THEN** the system SHALL exclude movies with viewCount of 0 from the candidate list

#### Scenario: Oldest-watched requires Plex
- **WHEN** the user runs `radarrer cleanup 5TB --strategy oldest-watched` without Plex configured
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
The system SHALL support a `--plex-section` flag on the cleanup command to specify which Plex library section to use for watch data, with auto-detection when only one movie library exists.

#### Scenario: Auto-detect section
- **WHEN** the user runs cleanup with a Plex-aware strategy and exactly one Plex movie library exists
- **THEN** the system SHALL automatically use that library section for watch data

#### Scenario: Specify section
- **WHEN** the user runs `radarrer cleanup 5TB --strategy never-watched --plex-section 2`
- **THEN** the system SHALL use Plex library section 2 for watch data

#### Scenario: Multiple sections without flag
- **WHEN** the user runs cleanup with a Plex-aware strategy and multiple Plex movie libraries exist without specifying `--plex-section`
- **THEN** the system SHALL display an error listing the available movie libraries and their keys

## MODIFIED Requirements

### Requirement: Deletion strategy selection
The system SHALL support five deletion strategies selectable via the `--strategy` flag: `oldest` (default), `oldest-added`, `never-watched`, `least-watched`, and `oldest-watched`.

#### Scenario: Oldest strategy (default)
- **WHEN** the user specifies `--strategy oldest` or omits the `--strategy` flag
- **THEN** the system SHALL sort candidate movies by their effective date ascending, where the effective date is the Plex last watched timestamp if available, otherwise the Radarr added date. Movies are interleaved regardless of whether they have Plex data.

#### Scenario: Oldest strategy without Plex
- **WHEN** the user uses the `oldest` strategy and Plex is not configured
- **THEN** the system SHALL sort all movies by their added date ascending (graceful degradation — no Plex data means all movies fall back to added date)

#### Scenario: Oldest strategy with Plex
- **WHEN** the user uses the `oldest` strategy and Plex is configured
- **THEN** the system SHALL fetch Plex watch data and sort movies using last watched date where available, falling back to added date for unmatched or unwatched movies

#### Scenario: Oldest added strategy
- **WHEN** the user specifies `--strategy oldest-added`
- **THEN** the system SHALL sort candidate movies by their `added` date in ascending order (oldest added first), ignoring any Plex watch data

#### Scenario: Never-watched strategy
- **WHEN** the user specifies `--strategy never-watched`
- **THEN** the system SHALL select only movies with a Plex viewCount of 0, sorted by added date ascending

#### Scenario: Least-watched strategy
- **WHEN** the user specifies `--strategy least-watched`
- **THEN** the system SHALL sort candidate movies by Plex viewCount ascending, then by lastViewedAt ascending

#### Scenario: Oldest-watched strategy
- **WHEN** the user specifies `--strategy oldest-watched`
- **THEN** the system SHALL filter to movies with viewCount > 0, sort by Plex lastViewedAt ascending (oldest watch first)

### Requirement: Cleanup output
The system SHALL display a summary of the cleanup operation including: disk space information, Radarr library size, threshold, space to free, and a table of movies selected for deletion with their title, size, added date, and watch/activity data.

#### Scenario: Cleanup summary display
- **WHEN** the cleanup command identifies movies for deletion
- **THEN** the system SHALL display a table with columns: Title, Size, Added Date, and activity/watch columns appropriate to the strategy and Plex availability

#### Scenario: No movies to delete
- **WHEN** the cleanup command determines no movies need to be deleted (usage is within threshold)
- **THEN** the system SHALL display the current disk usage, Radarr library size, and threshold and indicate no action is needed

## REMOVED Requirements

### Requirement: Least activity strategy (least-activity)
**Reason**: Superseded by Plex-based `least-watched` strategy which uses real playback data instead of unreliable Radarr download/import history events.
**Migration**: Use `--strategy least-watched` instead. Requires Plex configuration.

### Requirement: Oldest activity strategy (oldest-activity)
**Reason**: Replaced by Plex-based `oldest-watched` strategy which uses real last-played timestamps instead of Radarr history event dates.
**Migration**: Use `--strategy oldest-watched` instead. Requires Plex configuration.
