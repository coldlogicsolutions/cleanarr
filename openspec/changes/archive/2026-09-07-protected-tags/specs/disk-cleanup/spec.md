## MODIFIED Requirements

### Requirement: Cleanup respects movie exclusions
The system SHALL never delete movies or episodes carrying a protected tag. The protected set is the union of `protected_tags` in the `[cleanup]` config section, the comma-separated `RADARR_PROTECTED_TAGS` env var, and `--exclude-tag` values. Tag names are resolved case-insensitively against Radarr tags for movies and Sonarr tags for series; an episode is protected when its series carries a protected tag.

#### Scenario: Exclude tagged movies
- **WHEN** the user runs `radarrer cleanup 2TB --exclude-tag keep`
- **THEN** the system SHALL exclude all movies tagged "keep" in Radarr from the candidate list

#### Scenario: Exclude tagged series
- **WHEN** the user runs `radarrer cleanup 2TB --exclude-tag keep` with Sonarr configured
- **THEN** the system SHALL exclude every episode of any series tagged "keep" in Sonarr from the candidate list

#### Scenario: Configured protected tags
- **WHEN** the config file contains `[cleanup] protected_tags = ["keep"]` and the user runs `radarrer cleanup 2TB` with no `--exclude-tag`
- **THEN** the system SHALL exclude movies and series tagged "keep" exactly as if `--exclude-tag keep` had been given

#### Scenario: CLI flag is additive
- **WHEN** the config file protects "keep" and the user passes `--exclude-tag kids`
- **THEN** the system SHALL protect both "keep" and "kids"

#### Scenario: Protected tags shown in output
- **WHEN** any protected tags are in effect and the output format is table
- **THEN** the system SHALL print the effective protected tag names before the candidate table
