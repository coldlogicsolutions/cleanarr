## ADDED Requirements

### Requirement: Disk usage threshold check
The system SHALL query the Radarr disk space API and determine whether total disk usage exceeds a user-specified threshold.

#### Scenario: Disk usage below threshold
- **WHEN** the user runs `radarrer cleanup --max-size 2TB`
- **THEN** the system SHALL report that disk usage is within the threshold and take no further action

#### Scenario: Disk usage exceeds threshold
- **WHEN** disk usage exceeds the specified `--max-size` value
- **THEN** the system SHALL calculate how much space needs to be freed and proceed to identify candidate movies for deletion

### Requirement: Size threshold format
The system SHALL accept the `--max-size` flag with human-readable size values supporting units: B, KB, MB, GB, TB.

#### Scenario: Terabyte threshold
- **WHEN** the user specifies `--max-size 2TB`
- **THEN** the system SHALL interpret this as 2,000,000,000,000 bytes (decimal/SI units)

#### Scenario: Gigabyte threshold
- **WHEN** the user specifies `--max-size 500GB`
- **THEN** the system SHALL interpret this as 500,000,000,000 bytes

### Requirement: Deletion strategy selection
The system SHALL support three deletion strategies selectable via the `--strategy` flag: `oldest-added`, `least-activity`, and `oldest-activity`.

#### Scenario: Oldest added strategy
- **WHEN** the user specifies `--strategy oldest-added`
- **THEN** the system SHALL sort candidate movies by their `added` date in ascending order (oldest added first) and select movies for deletion starting from the oldest

#### Scenario: Least activity strategy
- **WHEN** the user specifies `--strategy least-activity`
- **THEN** the system SHALL sort candidate movies by their total number of history events in ascending order (fewest events first) and select movies for deletion starting from those with the least activity

#### Scenario: Oldest activity strategy
- **WHEN** the user specifies `--strategy oldest-activity`
- **THEN** the system SHALL sort candidate movies by the date of their most recent history event in ascending order (longest idle first) and select movies for deletion starting from those with the oldest last activity

### Requirement: Dry-run mode
The system SHALL default to dry-run mode for the cleanup command. Actual deletion SHALL require an explicit `--confirm` flag.

#### Scenario: Default dry-run
- **WHEN** the user runs `radarrer cleanup --max-size 2TB --strategy oldest-added` without `--confirm`
- **THEN** the system SHALL display the list of movies that would be deleted, their sizes, and the total space that would be freed, but SHALL NOT delete anything

#### Scenario: Confirmed deletion
- **WHEN** the user runs `radarrer cleanup --max-size 2TB --strategy oldest-added --confirm`
- **THEN** the system SHALL delete the identified movies including their files from disk via the Radarr API

### Requirement: Cleanup output
The system SHALL display a summary of the cleanup operation including: current disk usage, threshold, space to free, and a table of movies selected for deletion with their title, size, added date, and last activity date.

#### Scenario: Cleanup summary display
- **WHEN** the cleanup command identifies movies for deletion
- **THEN** the system SHALL display a table with columns: Title, Size, Added Date, Last Activity, and a summary line showing total space to be freed

#### Scenario: No movies to delete
- **WHEN** the cleanup command determines no movies need to be deleted (usage is within threshold)
- **THEN** the system SHALL display the current disk usage and threshold and indicate no action is needed

### Requirement: Cleanup respects movie exclusions
The system SHALL support a `--exclude-tag` flag that prevents movies with specified tags from being deleted during cleanup.

#### Scenario: Exclude tagged movies
- **WHEN** the user runs `radarrer cleanup --max-size 2TB --strategy oldest-added --exclude-tag keep`
- **THEN** the system SHALL exclude all movies tagged with "keep" from the candidate deletion list

### Requirement: Cleanup progress reporting
The system SHALL report progress during cleanup when deleting multiple movies.

#### Scenario: Deletion progress
- **WHEN** the cleanup command is deleting movies with `--confirm`
- **THEN** the system SHALL display progress for each deletion (e.g., `[3/10] Deleted "Movie Title" (4.2 GB freed)`)

### Requirement: Cleanup concurrency control
The system SHALL support a `--concurrency` flag to control how many concurrent API requests are made when fetching movie history data, defaulting to 5.

#### Scenario: Custom concurrency
- **WHEN** the user specifies `--concurrency 10`
- **THEN** the system SHALL make up to 10 concurrent history API requests when gathering activity data
